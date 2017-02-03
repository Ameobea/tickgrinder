//! The frontend of the SimBroker that is exposed to clients.  It contains the real `SimBroker` instance
//! internally, provides access to it via streams, and holds it in a thread during the simulation loop.

use super::*;
use futures::stream::BoxStream;
use futures::sync::mpsc::Sender;

/// The client-facing part of the SimBroker.  Implements the `Broker` trait and enables clients to communicate with
/// the underlying `SimBroker` instance while it's blocked on the simulation loop.
pub struct SimBrokerClient {
    /// The internal `SimBroker` instance before being consumed in the simulation loop
    simbroker: SimBroker,
    /// The channel over which messages are passed to the inner `SimBroker`
    inner_tx: mpsc::SyncSender<(BrokerAction, Complete<BrokerResult>)>,
    /// A handle to the receiver for the channel through which push messages are received
    push_stream_recv: Option<(BoxStream<(u64, BrokerResult), ()>, Arc<AtomicBool>,)>,
    /// Holds the tick channels that are distributed to the clients
    tick_recvs: HashMap<String, (BoxStream<Tick, ()>, Arc<AtomicBool>,)>,
}

impl Broker for SimBrokerClient {
    fn init(settings: HashMap<String, String>) -> Oneshot<Result<Self, BrokerError>> {
        let (c, o) = oneshot::<Result<Self, BrokerError>>();
        // this currently panics if you give it bad values...
        // TODO: convert FromHashmap to return a Result<SimbrokerSettings>
        let broker_settings = SimBrokerSettings::from_hashmap(settings);
        let cs = CommandServer::new(Uuid::new_v4(), "Simbroker");
        // the channel to communicate commands to the consumed broker.
        // has a buffer size of 32 to avoid blocking during the send cycle
        let (tx, rx) = mpsc::sync_channel(32);
        let mut sim = match SimBroker::new(broker_settings, cs, rx) {
            Ok(sim) => sim,
            Err(err) => {
                c.complete(Err(err));
                return o;
            },
        };

        let push_stream_recv = sim.push_stream_recv.take().expect("No push stream to take from the sim!");
        let mut tick_hm = HashMap::new();
        // take the tick receivers from each of the symbols and put them in the `HashMap`
        for sym in sim.symbols.iter_mut() {
            let recv = sym.client_receiver.take().unwrap();
            tick_hm.insert(sym.name.clone(), (recv, Arc::new(AtomicBool::new(false)),));
        }

        let mut client = SimBrokerClient {
            simbroker: sim,
            inner_tx: tx,
            push_stream_recv: Some((push_stream_recv, Arc::new(AtomicBool::new(false)),)),
            tick_recvs: tick_hm,
        };
        // init the simulation loop in another thread
        client.init_sim_loop().expect("Unable to start the simulation loop!");

        c.complete(Ok(client));

        o
    }

    fn get_ledger(&mut self, account_id: Uuid) -> Oneshot<Result<Ledger, BrokerError>> {
        let (complete, oneshot) = oneshot::<Result<Ledger, BrokerError>>();
        let account = self.simbroker.get_ledger_clone(account_id);
        complete.complete(account);

        oneshot
    }

    fn list_accounts(&mut self) -> Oneshot<Result<HashMap<Uuid, Account>, BrokerError>> {
        let (complete, oneshot) = oneshot::<Result<HashMap<Uuid, Account>, BrokerError>>();
        complete.complete(Ok(self.simbroker.accounts.data.clone()));

        oneshot
    }

    fn execute(&mut self, action: BrokerAction) -> PendingResult {
        // push the message into the inner `SimBroker`'s simulation queue
        let (complete, oneshot) = oneshot::<BrokerResult>();
        self.inner_tx.try_send((action, complete)).expect("Unable to send through inner_tx");
        oneshot
    }

    /// Maps a new channel through the pushtream, duplicating all messages sent to it.
    fn get_stream(&mut self) -> Result<BoxStream<(u64, BrokerResult), ()>, BrokerError> {
        let (fork_tx, fork_rx) = channel(0);
        // move out the forked tx and the `AtomicBool` indicating whether or not it's consumed
        let (strm, old_bool) = self.push_stream_recv.take().unwrap();
        // set the old `AtomicBool` to `true` since we're handing the old fork to the client for consumption
        old_bool.store(true, Ordering::Relaxed);

        let mut tx_opt = Some(fork_tx);
        let atomb = Arc::new(AtomicBool::new(false));
        let atomb_clone = atomb.clone();
        let new_tail = strm.map(move |msg| {
            // only send the tick to the fork if the fork has been consumed as indicated by `atomb_clone`
            if atomb_clone.load(Ordering::Relaxed) {
                // unfortunate workaround needed since `send()` takes `self`
                let mut tx = tx_opt.take().unwrap();
                tx = tx.send(msg.clone()).wait().unwrap();
                tx_opt = Some(tx);
            }
            msg
        });
        // move the new fork back into self along with a new `AtomicBool` set to false since the new
        // fork isn't yet consumed.
        self.push_stream_recv = Some((fork_rx.boxed(), atomb,));

        // hand off the new tail to the client with the assumption that they will drive it to completion.
        Ok(new_tail.boxed())
    }

    fn sub_ticks(&mut self, symbol: String) -> Result<BoxStream<Tick, ()>, BrokerError> {
        if self.tick_recvs.get(&symbol).is_none() {
            return Err(BrokerError::NoSuchSymbol);
        }

        // take the tickstream out of the `HashMap` so we can modify it
        let (tickstream, old_bool) = self.tick_recvs.remove(&symbol).unwrap();
        // set the old boolean to true since we're sending off the stream to the client to be consumed
        old_bool.store(true, Ordering::Relaxed);
        let (fork_tx, fork_rx) = channel(0);

        let mut tx_opt = Some(fork_tx);
        let atomb = Arc::new(AtomicBool::new(false));
        let atomb_clone = atomb.clone();
        let new_tickstream = tickstream.map(move |tick| {
            // check to make sure that the tickstream is consumed before sending ticks down it.
            // Since this is a bounded channel, sending ticks down it without a client waiting at the other
            // end will cause the ENTIRE future tree to block on the `send()` call.
            if atomb_clone.load(Ordering::Relaxed) {
                let mut tx = tx_opt.take().unwrap();
                tx = tx.send(tick).wait().unwrap();
                tx_opt = Some(tx);
            }
            tick
        });
        // put the forked tickstream back in the `HashMap` along with a new `AtomicBool` set to false since
        // the new fork tickstream is not yet consumed.
        self.tick_recvs.insert(symbol, (fork_rx.boxed(), atomb,));

        // return the new tail tickstream to the client with the assumption that it will be driven to completion there.
        Ok(new_tickstream.boxed())
    }

    /// This usually allows for a custom message to be sent to the broker to fulfill a unique functionality not
    /// covered by the rest of the trait functions.  For the simbroker, this is used to drive progress on the internal
    /// simulation loop.
    fn send_message(&mut self, code: usize) -> usize {
        self.simbroker.tick_sim_loop(code)
    }
}

impl SimBrokerClient {
    /// Initializes the inner `SimBroker` and starts its simulation loop.  This essentially "turns on" the
    /// `SimBroker`.  After this is called, it's impossible to do things like add new symbols.
    pub fn init_sim_loop(&mut self) -> BrokerResult {
        self.simbroker.init_sim_loop();

        // fork the push stream internally so we can drive progress on the tail while still getting clones during sim
        // remove the tickstream from the `Option`
        let (push_stream, false_abool) = self.push_stream_recv.take().unwrap();
        // fork that will be replaced in the `HashMap` and forked again as needed during operation
        let (fork_tx, fork_rx) = channel(0);
        let mut fork_tx_opt = Some(fork_tx);
        // perform the fork by mapping the fork into the parent
        let abool_arc_clone = false_abool.clone();
        let tail_pushstream = push_stream.map(move |msg| {
            // Only send the tick to the fork (`SimBrokerClient`'s pushstream) if at least one strategy process has
            // taken a copy.  This allows the simulation process to start and the strategy to become interested in it
            // in response to some event.
            if abool_arc_clone.load(Ordering::Relaxed) {
                let tx = fork_tx_opt.take().unwrap();
                let new_tx = tx.send(msg.clone()).wait().unwrap();
                fork_tx_opt = Some(new_tx);
            }
            msg
        });
        self.push_stream_recv = Some((fork_rx.boxed(), false_abool,));

        // get all of the tickstreams ready for the simulation process to start
        let mut keys = Vec::new();
        for k in self.tick_recvs.keys() {
            keys.push(k.clone());
        }
        let (tickstream_tx, tickstream_rx) = channel(0);
        let mut tickstream_tx_opt = Some(tickstream_tx);
        // fork each of the tick receivers so we can consume the tail and still get copies during simulation
        for name in keys {
            let tickstream_tx: Sender<_> = tickstream_tx_opt.take().unwrap();
            // fork that will be replaced in the `HashMap` and forked again as needed during operation
            let (fork_tx, fork_rx) = channel(0);
            // remove the tickstream from the `HashMap`
            let (tickstream, false_abool) = self.tick_recvs.remove(&name).unwrap();
            let mut fork_tx_opt = Some(fork_tx);
            // perform the fork by mapping the forked stream into the parent stream
            let abool_arc_clone = false_abool.clone();
            let tail_tickstream = tickstream.map(move |t| {
                // Only send the tick to the fork (`SimBrokerClient`'s tickstream) if at least one strategy process has
                // taken a copy.  This allows the simulation process to start and the strategy to become interested in it
                // in response to some event.
                if abool_arc_clone.load(Ordering::Relaxed) {
                    let tx = fork_tx_opt.take().unwrap();
                    let new_tx = tx.send(t).wait().unwrap();
                    fork_tx_opt = Some(new_tx);
                }
                t
            });
            // re-insert the forked tickstream into the `HashMap`
            self.tick_recvs.insert(name, (fork_rx.boxed(), false_abool,));
            let new_tickstream_tx = tickstream_tx.send(tail_tickstream.boxed()).wait().unwrap();
            tickstream_tx_opt = Some(new_tickstream_tx);
        }

        // thread in which all of the tickstreams are consumed.  This drives them to completion so all of their
        // forks (which have been handed off to clients) are populated with values.
        thread::spawn(move || {
            let tickstreams_comb = tickstream_rx.flatten();
            for _ in tickstreams_comb.wait() {
                // do nothing; we're just consuming the streams.
            }
        });

        // thread in which the push stream is consumed.  This drives it to completion for all clients that took forks of it.
        thread::spawn(move || {
            for _ in tail_pushstream.wait() {
                // do nothing; we're just consuming the stream;
            }
        });

        Ok(BrokerMessage::Success)
    }

    /// Calls same function on inner `SimBroker`
    pub fn oneshot_price_set(
        &mut self, name: String, price: (usize, usize), is_fx: bool, decimal_precision: usize,
    ) -> BrokerResult {
        self.simbroker.oneshot_price_set(name, price, is_fx, decimal_precision);
        Ok(BrokerMessage::Success)
    }
}

#[test]
fn stream_forking() {
    use futures::sync::mpsc::channel;

    let (mut tx_head, rx_head) = channel(0);
    let (tx_fork, rx_fork) = channel(0);

    let mut tx_fork_opt = Some(tx_fork);
    let rx_tail = rx_head.map(move |x| {
        let tx = tx_fork_opt.take().unwrap();
        let new_tx = tx.send(x).wait().unwrap();
        tx_fork_opt = Some(new_tx);
        x
    });

    thread::spawn(move || {
        loop {
            tx_head = tx_head.send(0).wait().unwrap();
        }
    });

    // drive the whole thing to completion by waiting on the tail
    thread::spawn(move || {
        for _ in rx_tail.wait() {

        }
    });

    // make sure that we're receiving messages on the fork
    let v: Vec<Result<usize, ()>> = rx_fork.wait().take(10).collect();
    println!("{:?}", v);
    assert_eq!(v.len(), 10);
}
