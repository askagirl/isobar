process.env.NODE_ENV = "production";

const { App, buildViewRegistry } = require("isobar_web");
const IsobarClient = require("../shared/isobar_client");
const QueryString = require("querystring");
const React = require("react");
const ReactDOM = require("react-dom");
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
