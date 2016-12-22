# Private Directory

This directory contains all of the custom, proprietary code for the platform that's meant to be supplied by the user.  It should be maintained as a separate git repository and, in general, kept private.  It compiles down to a library that can be plugged into the main platform in order to implement custom functionality.

## Overview

The platform is just that - a platform on top of which you can build a functioning trading system.  Due to the fact that successful trading strategies are by their very nature secret, this functionality has been built in so that you can use and contribute to the public platform without having to sacrifice the security and privacy of your own strategies, indicators, and data analysis techniques.

## Usage

There are several subdirectories inside this directory that each contain files pertaining to a different proprietary part of the algorithmic trading process.  By default, they are symlinked into the platform's code directories so that their contents are automatically compiled into the platform during build.

### Strategies
Strategies are the main logic for the trading system.  It is their job to ingest live data from the platform and use it to create trade conditions which are in turn fed to the Tick Processors.

### Indicators
Indicators are rules for transforming data.  Some examples of classic trading indicators include the SMA, RSI, and Stochastic.  You can write your own indicators (which can also be used for plotting stored data).
