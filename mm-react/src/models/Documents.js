//! Functions for interacting with the document storage engine.

import { message } from 'antd';

const d = new Date();

export default {
  namespace: 'documents',

  state: {
    queryResults: [], // list of all document titles returned in response to a query
    docQueryCbs: [], // list of functions that are called with the list of matched titles every time a query response is received
    returnedDoc: { // the document returned from a query for a document
      title: 'Placeholder Document',
      body: 'Select a document to display by using the search box above.  When you select one, it will be displayed here.',
      tags: [],
      creation_date: '' + d.getTime(),
      modification_date: '' + d.getTime(),
    },
  },

  reducers: {
    /**
     * Registers a callback to be executed every time a new `DocQueryResponse` is received.
     */
    registerDocQueryReceiver (state, {cb}) {
      return {...state,
        docQueryCbs: [...state.docQueryCbs, cb]
      };
    },

    /**
     * Called when a doc query response is received from the spawner.  Updates the stored list of matched titles and
     * executes all stored callbacks.
     */
    docQueryResponseReceived (state, {msg}) {
      let matchedDocs;
      if(msg.res.DocumentQueryResult) {
        matchedDocs = msg.res.DocumentQueryResult.results.map(o => JSON.parse(o).title);

        // execute all registered callbacks
        for(var i=0; i<state.docQueryCbs.length; i++) {
          state.docQueryCbs[i](matchedDocs);
        }

        return {...state,
          queryResults: matchedDocs,
        };
      } else if(msg.res.Error) {
        message.error('Error while processing query: ' + msg.res.Error.status);
      } else {
        message.error('Unknown error occured while processing query: ' + JSON.stringify(msg));
      }

      return {...state};
    },

    /**
     * Called when a response is received from a request to save a document.
     */
    documentStoreResponseReceived (state, {msg}) {
      if(msg.res == 'Ok') {
        message.success('Document successfully saved.');
      } else if(msg.res.Error) {
        message.error('Error saving document: ' + msg.res.Error.status);
      } else {
        message.error('Unhandled error occured while tring to save document: ' + JSON.stringify(msg));
      }

      return {...state};
    },

    /**
     * Called when the response of a request for a document is received.
     */
    documentRequestResultReceived (state, {msg}) {
      if(msg.res.Document) {
        return {...state,
          returnedDoc: msg.res.Document.doc,
        };
      } else if(msg.res.Error) {
        message.error('Error while fetching document from store: ' + msg.res.Error.status);
      } else {
        message.error('Unexpected response received while fetching document from store: ' + JSON.stringify(msg));
      }

      return {...state};
    }
  },

  effects: {
    /**
     * Sends a document query to the Tantivy-backed document store.  Registers interest in responses with the UUID
     * of the sent query and handles the responses by updating the `queryResults` state.
     */
    *sendDocQuery ({query}, {call, put}) {
      let cmd = {QueryDocumentStore: {query: query}};
      yield put({
        type: 'platform_communication/sendCommandToInstance',
        cb_action: 'documents/docQueryResponseReceived',
        cmd: cmd,
        instance_name: 'Spawner',
      });
    },

    /**
     * Given the content of a CKEditor document, saves it in the document store
     */
    *saveDocument ({title, tags, body}, {call, put}) {
      let d = new Date();
      let doc = {title: title, tags: tags, body: body, creation_date: d.getTime() + '', modification_date: d.getTime() + ''};
      let cmd = {InsertIntoDocumentStore: {doc: JSON.stringify(doc)}};

      yield put({
        type: 'platform_communication/sendCommandToInstance',
        cb_action: 'documents/documentStoreResponseReceived',
        cmd: cmd,
        instance_name: 'Spawner',
      });
    },

    /**
     * Asks the document store to return the document with a certain title.
     */
    *requestDocument ({title}, {call, put}) {
      let cmd = {GetDocument: {title: title}};

      yield put({
        type: 'platform_communication/sendCommandToInstance',
        cb_action: 'documents/documentRequestResultReceived',
        cmd: cmd,
        instance_name: 'Spawner',
      });
    }
  },
};
