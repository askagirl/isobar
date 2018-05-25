process.env.NODE_ENV = "production";

const { React, ReactDOM, App, buildViewRegistry } = require("isobar_ui");
const IsobarClient = require("../shared/isobar_client");
const QueryString = require("querystring");
const $ = React.createElement;

async function start() {
  const url = window.location.search.replace("?", "");
  const { socketPath, windowId } = QueryString.parse(url);

  const isobarClient = new IsobarClient();
  await isobarClient.start(socketPath);
  const viewRegistry = buildViewRegistry(isobarClient);

  let initialRender = true;
  isobarClient.addMessageListener(message => {
    switch (message.type) {
      case "UpdateWindow":
        ViewRegistry.update(message);
        if (initialRender) {
          ReactDOM.render(
            $(App, { viewRegistry }),
            document.getElementById("app")
          );
          initialRender = false;
        }
        break;
      default:
        console.warn("Received unexpected message", message);
    }
  });

  isobarClient.sendMessage({
    type: "StartWindow",
    window_id: Number(windowId),
    height: window.innerHeight
  });
}

start();
