# SimBroker

The **SimBroker** is a tick-accurate simulated broker that is designed to accurately test strategies using historical data and mimic real broker behaviour as closely as possible.

SimBrokers are spawned and managed by the **Backtester** and are fed data from a running backtest.  Internally, the can manage data for multiple symbols coming from multiple sources and ensure that it is relayed to connected strategies in the correct order.  They are designed to implement the full `Broker` trait, accepting all the order types and commands that real brokers do.  In addition, they keep track of simulated accounts including balances, open/closed orders and positions, and profits.

The SimBroker contains a wide range of configuration options that allow it to be fine-tuned to replicate the behavior any existing broker.  Options exist for simulating network delay and order processing delay without having to actually block for those delays themselves.  In addition, the broker can be configured to automatically, dynamically calculate position values, buying power, profit, and other values so that results remain as accurate as possible.

The SimBroker also gathers statistics about the performance of strategies based on their interactions with it and can be configured to log in precise detail the exact order in which events occured.  This functionality is used by the Backtester and the MM in order to provide detailed information about strategies and their performance.

## Implementation
Since the TickGrinder platform is designed to be event-based with trading actions only taking place in response to new data being received, it is possible to simulate trading activity at a very fast rate while still maintaining real-time accuracy.  In addition to internal synchronization and ordering mechanisms used to ensure that events are executed in the order they are received.

To simulate things like processing delay and network latency while preserving non-blocking behavior, an internal event loop built on top of a priority queue is used.  All historical ticks, broker commands, and outgoing messages are inserted into this queue with their timestamps adjusted for simulated latency so that everything happens in precise order.

## Development
The SimBroker is currently undergoing active development.  It is not yet functional and mahy of the features described above may not be fully implemented in this current release.  I want to have a full battery of tests in place to verify its integrity and accuracy before releasing it officially.
