# wety-client

This is the frontend for [`wety.org`](https://www.wety.org/). See the root directory of the [`wety`](https://github.com/jmviz/wety) repository for the data processing and API server code.

## Local development

For local development, you'll need to set up both the `wety` API server and the `wety-client` server. Follow the instructions in the README in the root directory of this repository to set up and run the API server.

If you don't have `node` and `npm` installed, [install them](https://docs.npmjs.com/downloading-and-installing-node-js-and-npm). Clone this repo (if you haven't already) and `cd` into the `client` subdirectory. Then install the dependencies:

```bash
npm install
```

You can then start the client server:

```bash
npm start
```

This will generate a development build and start a local server at `127.0.0.1:8000`. Any changes you make to the source files will be hot-reloaded.
