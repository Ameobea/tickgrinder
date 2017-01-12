# TickGrinder Backtester
The backtester is an external application that integrates into the TickGrinder platform and simulates live market data in order to test the platform's trading activities.  Its primary purpose is to allow users to test out strategies and view their performance based on historical data downloaded elsewhere.

# Backtest Modes
There are different modes that the backtester can use, each acting a bit differently than the others.  This allows for the backtester to be used in unique situations where different parts of the platform's performance are important.

# Live Mode
Live mode attempts to simulate live trading conditions as closely as possible.  It reads ticks out at the rate that they were recorded, simulating the exact conditions that would be experienced in a live trading environment.  This main purpose of this mode is to verify platform integrity and verify that it acts as expected in a live environment.

# Fast Mode
In Fast Mode, ticks are sent through the system as fast as it can handle them.  The ideas is that the platform will still act in the same way it would in a live trading environment but the rate at which it processes the data is greatly amplified.  This mode is best suited for profiling strategies and determining their runtime characteristics, profitability, and other statistics.
