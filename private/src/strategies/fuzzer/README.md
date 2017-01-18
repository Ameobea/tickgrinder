# Broker Fuzzer
The broker fuzzer is not so such a strategy as a tool used for testing the functionality of the platform's broker shims.  It was originally designed to verify the functionality of the Backtester module's SimBroker

## Functionality
Traditional fuzzing works by trying to find issues, bugs, or vulnerabilities in programs by subjecting them to a large amount of semi-random inputs with the goal of making it break or misbehave in some way.

The broker fuzzer works in a similar way.  It genereates a large number of messages that it then sends to the specified broker.  These messages consist of all possible trading actions, status queries, and other broker commands that are available through the Broker API.  In addition, it will log in precise detail the order of events that take place from its view.  The fuzzer can be configured to be deterministic in its fuzzing activities so that particular tests can be repeated exactly.  When the log output from the fuzzer is compared to the advanced logs produced by the SimBroker itself, it's possible to verify that events happen in precisely the right order and that no race conditions or unordered events take place.

## Usage
The Fuzzer is still in active development and is currently not ready for real use.  This section will be updated once development has progressed to the point that the tool is ready for use.  // TODO
