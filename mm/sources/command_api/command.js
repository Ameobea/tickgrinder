//! Functions for programatically executing commands and handling their responses.
var JSONExp = require('jsonexp');

// Table for holding the UUID->Names of all running instances
var instance_table = {};

/// Initializes the instance table from the Spawner
function init(){
  // TODO
}

/// Sends the given command (As a JS Object), then returns a promise that fulfills to
/// the match object of the first response if it doesn't match to null.  If it matches
/// to null or no responses are received within the timeout window, the promise rejects.
function query(command, channel)
