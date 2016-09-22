# FXCM Tick Downloader
This tool integrates with a remote Java server that serves as a link between the FXCM API and this NodeJS application.  The Java application can be found in the `tick_recorder` directory.  This tool and the Java server are linked using Redis pubsub and all data is passed over a shared channel.

## Download Method
The tool requests ticks in 10-second chunks which are queued up all at once, the default being 50 10-second chunks at a time.  These requests are relayed through the Java application to the FXCM API servers and return the responses over the pubsub channel as soon as they arrive.  If no response is received in a certain amount of time (default 300ms), the missing chunks are resent.

Each chunk is assigned a `uuid` before being transmitted to the server which is sent back along with the response since responses do not come in order.  The server sends a response (`type: segmentID`) as soon as it receives the request that contains the The server also creates an `id` for the request that is used for identification if the request fails (the failure doesn't preserve the `uuid`).

Once a result is received from the FXCM API, it is relayed back to the client (`type: segment`) and stored.  The segment is marked as successful so it is not resent.

## Error handling
For cases where no ticks exist within a requested range, the error (`No ticks in range`) is transmitted to the client along with the FXCM request ID.  This is matched with the UUID which is then marked as successful so it isn't retransmitted.

## Processing result data
The result data is not sorted and may contain duplicate rows.  It should be sorted in order of the first column (timestamp) and duplicate rows should be removed.  This can be done easily with Linux command line tools such as `sort`; `sort usdcad.csv | uniq -u > usdcad_sorted.csv` should do the trick.
