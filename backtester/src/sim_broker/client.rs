//! The frontend of the SimBroker that is exposed to clients.  It contains the real `SimBroker` instance
//! internally, provides access to it via streams, and holds it in a thread during the simulation loop.

use super::*;

/// The client-facing part of the SimBroker.  Implements the `Broker` trait and enables clients to communicate with
/// the underlying `SimBroker` instance while it's blocked on the simulation loop.
pub struct SimBrokerClient {
    /// The internal `SimBroker` instance before being consumed in the simulation loop
    simbroker: Option<SimBroker>,
    /// The channel over which messages are passed to the inner `SimBroker`
    inner_tx: UnboundedSender<(BrokerAction, Complete<BrokerResult>)>,
}

impl Broker for SimBrokerClient {
    fn init(settings: HashMap<String, String>) -> Oneshot<Result<Self, BrokerError>> {
        let (c, o) = oneshot::<Result<Self, BrokerError>>();
        // this currently panics if you give it bad values...
        // TODO: convert FromHashmap to return a Result<SimbrokerSettings>
        let broker_settings = SimBrokerSettings::from_hashmap(settings);
        let cs = CommandServer::new(Uuid::new_v4(), "Simbroker");
        let (tx, rx) = unbounded();
        let sim = SimBroker::new(broker_settings, cs, rx);
        let client = SimBrokerClient {
            simbroker: Some(sim),
            inner_tx: tx,
        };

        c.complete(Ok(client));

        o
    }

    fn get_ledger(&mut self, account_id: Uuid) -> Oneshot<Result<Ledger, BrokerError>> {
        let (complete, oneshot) = oneshot::<Result<Ledger, BrokerError>>();
        let simbroker_res = self.get_simbroker();
        if simbroker_res.is_err() {
            complete.complete(Err(simbroker_res.err().unwrap()));
            return oneshot
        }
        let simbroker = simbroker_res.unwrap();
        let account = simbroker.get_ledger_clone(account_id);
        complete.complete(account);

        oneshot
    }

    fn list_accounts(&mut self) -> Oneshot<Result<HashMap<Uuid, Account>, BrokerError>> {
        let (complete, oneshot) = oneshot::<Result<HashMap<Uuid, Account>, BrokerError>>();
        let simbroker_res = self.get_simbroker();
        if simbroker_res.is_err() {
            complete.complete(Err(simbroker_res.err().unwrap()));
            return oneshot
        }
        let simbroker = simbroker_res.unwrap();
        complete.complete(Ok(simbroker.accounts.clone()));

        oneshot
    }

    fn execute(&mut self, action: BrokerAction) -> PendingResult {
        // push the message into the inner `SimBroker`'s simulation queue
        let (complete, oneshot) = oneshot::<BrokerResult>();
        let inner_tx = &self.inner_tx;
        inner_tx.send((action, complete)).expect("Unable to send through inner_tx");
        oneshot
    }

    fn get_stream(&mut self) -> Result<Box<Stream<Item=BrokerResult, Error=()> + Send>, BrokerError> {
        let simbroker = self.get_simbroker()?;
        if simbroker.push_stream_recv.is_none() {
            // TODO: Enable multiple handles to be taken?
            return Err(BrokerError::Message{
                message: "You already took a handle to the push stream and can't take another.".to_string()
            })
        }

        Ok(simbroker.push_stream_recv.take().unwrap().boxed())
    }

    fn sub_ticks(&mut self, symbol: String) -> Result<Box<Stream<Item=Tick, Error=()> + Send>, BrokerError> {
        let simbroker = self.get_simbroker()?;

        if !simbroker.symbols.contains(&symbol) {
            return Err(BrokerError::NoSuchSymbol);
        }

        let mut sym = &mut simbroker.symbols[&symbol];
        if sym.client_receiver.is_some() {
            Ok(Box::new(mem::replace(&mut sym.client_receiver, None).unwrap()))
        } else {
            return Err(BrokerError::Message{
                message: "You already took a handle to the tick stream for that symbol and can't take another.".to_string()
            })
        }
    }
}

impl SimBrokerClient {
    /// Helper function that tries to get a mutable reference to the inner `SimBroker`, returning a `BrokerError`
    /// if it has already been consumed in the simulation loop.
    fn get_simbroker(&mut self) -> Result<&mut SimBroker, BrokerError> {
        if self.simbroker.is_none() {
            return Err(BrokerError::Message{
                message: String::from("The SimBroker has already been initialized; you can't sub ticks now!"),
            });
        }
        Ok(self.simbroker.as_mut().unwrap())
    }

    /// Calls this function on the internal `SimBroker`
    pub fn register_tickstream(
        &mut self, name: String, raw_tickstream: UnboundedReceiver<Tick>, is_fx: bool, decimal_precision: usize
    ) -> BrokerResult {
        let simbroker = self.get_simbroker()?;
        simbroker.register_tickstream(name, raw_tickstream, is_fx, decimal_precision)
    }

    /// Initializes the inner `SimBroker` and starts its simulation loop.  This essentially "turns on" the
    /// `SimBroker`.  After this is called, it's impossible to do things like add new symbols.
    pub fn init_sim_loop(&mut self) -> BrokerResult {
        let simbroker_opt = self.simbroker.take();
        if simbroker_opt.is_none() {
            return Err(BrokerError::Message{
                message: String::from("The SimBroker has already been initialized!"),
            });
        }
        let simbroker = simbroker_opt.unwrap();

        thread::spawn(move || {
            // block this thread on the `SimBroker`'s simulation loop
            simbroker.init_sim_loop();
        });

        Ok(BrokerMessage::Success)
    }

    /// Calls same function on inner `SimBroker`
    fn oneshot_price_set(
        &mut self, name: String, price: (usize, usize), is_fx: bool, decimal_precision: usize,
    ) -> BrokerResult {
        let simbroker = self.get_simbroker()?;
        simbroker.oneshot_price_set(name, price, is_fx, decimal_precision);
        Ok(BrokerMessage::Success)
    }
}
