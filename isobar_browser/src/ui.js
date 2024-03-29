import { React, ReactDOM, App, buildViewRegistry } from "isobar_ui";
import IsobarClient from "./client";
const $ = React.createElement;

const client = new IsobarClient(new Worker("worker.js"));
const websocketURL = new URL("/ws", window.location.href);
websocketURL.protocol = "ws";
client.sendMessage({ type: "ConnectToWebsocket", url: websocketURL.href });

const viewRegistry = buildViewRegistry(client);

let initialRender = true;
client.onMessage(message => {
  switch (message.type) {
    case "UpdateWindow":
      viewRegistry.update(message);
      if (initialRender) {
        ReactDOM.render(
          $(App, { inBrowser: true, viewRegistry }),
          document.getElementById("app")
        );
        initialRender = false;
      }
      break;
    default:
      console.warn("Received unexpected message", mesage);
  }
});
