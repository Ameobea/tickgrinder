# TickGrinder Algorithmic Trading Platform

[![Join the chat at https://gitter.im/TickGrinder/Lobby](https://badges.gitter.im/TickGrinder/Lobby.svg)](https://gitter.im/TickGrinder/Lobby?utm_source=badge&utm_medium=badge&utm_campaign=pr-badge&utm_content=badge)
![](https://camo.githubusercontent.com/79318781f189b2ee91c3a150bf27813c386afaf2/68747470733a2f2f696d672e736869656c64732e696f2f62616467652f72757374632d6e696768746c792d79656c6c6f772e737667)
![](https://tokei.rs/b1/github/Ameobea/tickgrinder)
![](https://tokei.rs/b1/github/Ameobea/tickgrinder?category=files)

TickGrinder is a high performance algorithmic trading platform written primarilly in Rust.  It is designed with the goal of efficiently processing event-based market data as quickly as possible in order to automatically place and manage trades.

# Overview
The basis of the platform is written in Rust.  It consists of several distinct modules that operate independently but communicate with each other via a custom messaging protocol implemented on top of Redis Pub/Sub.  It is designed to be extensible and robust, capable of being used to trade any market consisting of event-based streaming Tick data.

Disclaimer: **This platform is currently in active, early pre-alpha development and is in no way ready for any kind of trading.**  I do not take any responsibility for your trading actions using this platform nor any financial losses caused by errors in this application.

## Tick Processors
The primary module is the Tick Processor.  Multiple tick processors can be spawned, one for each symbol/data stream that is being processed by the platform.  Their purpose is to convert live data into trading signals as quickly as possible.

Each time a tick arrives, a series of conditions are evaluated.  These conditions can be anything: a SMA crossing a threshold, the current timestamp being greater than a certain number, any evaluable expression that you can program can serve as a condition.  These conditions are designed to be dynamically set by the Optimizer module during trading operations.

## Optimizer/Strategies
The Optimizer is a module that controls the conditions evaluated by the Tick Processors to open trades and interact with the broker API.  Only one Optimizer is meant to run at once and it interacts with all Tick Processors that may be alive in the platform.

Optimizers, using whatever strategies you define, set the parameters and enable/disable trading conditions on the Tick Processors dynamically.  They accomplish this by sending and receiving Commands to and from the Tick Processors, interacting with the database, or using any external data sources you may find useful.  This application currently uses PostgreSQL as the main storage backend.

The strategies that are evaluted by the optimizer are meant to be written by you, the user!  I'm keeping my strategies secret, but I will provide a sample strategy in the future for reference.

## MM (Management/Monitoring) Interface
The MM is the interface between the platform and the user.  It contains controls for manually managing platform components, monitoring bot activity, starting backtests, and pretty much everything else.  It exists in the format of a simple NodeJS Express webserver and talks with the main communication system of the platform using a Websocket<->Redis bridge system.

It includes a custom charting module using Highcharts that can be extended with custom indicators to produce truly specialized charts for bot data.  The configurations for these charts can be compiled down into JSON-encoded Macro strings which can then be loaded to instantly re-create charts with unique data sources and settings.

## Broker APIs + Data Downloaders
Several scripts/utilities are included to interact with the FXCM Foreign Exchange API to create a live data feed and access historical market data.  Using scripts located in the `tick_writer` and `data_downloaders` directories, this application can be controlled through Redis and used as a primary data source for the trading platform.

The data downloader script saves tick data for a currency pair into CSV files, downloading chunks of data automatically.  The tick writer script does the same thing but processes live ticks sent over Redis instead.  As I mentioned before, more detailed documentation can be found in the respective directories.

# Installation
**Requirements**:
* Rust/Cargo Nightly
* NodeJS
* Redis
* PostgreSQL

After cloning the repository, you'll need to copy all instances of files named `conf.default.rs`, `conf.sample.js`, or anything similar to `conf.js/rs` in the same directory and fill out their values as appropriate.

After ensuring that you have a Redis and PostgreSQL server running and accessible to the program, you can make sure that everything is set up correctly by running `make test` in the root directory of the project.  This will automatically pull down all needed dependencies, attempt to build all modules, and run all included tests.  Any encountered errors will be printed to the console and you can use them to debug any issues you're having with the setup.

To use the live trading and data downloading functionatlities, you'll need to set up the FXCM Java Application (located in the TickRecorder directory).  Detailed setup and installation instructions for that are located in that folder.  Documentation for the data downloader and `tick_writer` scripts can also be found in their respective directories.

As of the current version (`a44581b`), all tests pass and the platform builds on both Linux and Windows using the current version of nightly Rust.  However, due to the extremely active development of the project and of the Rust nightlies/dependencies, that's very unlikely to be the case for long.  Sit tight and, if you find an issue, think about contributing!

# Contributing
This is an open source project project, so you're more than welcome to fork it and customize it for your own needs.

As for contributing to master, I'm very happy to merge pull requests containing syntax improvements, bugfixes, or small stuff like that.  However, for larger things such as rewriting large segments of code or adding new features, please file an Issue or contact me privately before putting a lot of work in.  This is my personal project and I want to maintain my position in absolute control of its development.

# Closing Remarks
If you've got any feedback or comments on the project, I'd love to hear it!  I'm always working on developing my skills as a programmer, so any sage advice from seasoned veterans (or questions from eager beginners) are very welcome.

If you find this project useful, exciting, or have plans to use this in production, **please** let me know!  I'd maybe be willing to work with you to make sure that your needs are met and improve the platform in the process.
