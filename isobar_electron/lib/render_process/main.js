process.env.NODE_ENV = "production";

const App = require("isobar_web/lib/app");
const FileFinder = require("isobar_web/lib/file_finder");
const QueryString = require("querystring");
const React = require("react");
const ReactDOM = require("react-dom");
const ViewRegistry = require("isobar_web/lib/view_registry");
const Workspace= require("isobar_web/lib/workspace");
const TextEditorView = require("isobar_web/lib/text_editor/text_editor");
const IsobarClient = require("../shared/isobar_client");
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

function buildViewRegistry(client) {
  const viewRegistry = new ViewRegistry({
    onAction: action => {
      action.type = "Action";
      client.sendMessage(action);
    }
  });
  viewRegistry.addComponent("Workspace", Workspace);
  viewRegistry.addComponent("FileFinder", FileFinder);
  viewRegistry.addComponent("BufferView", TextEditorView);
  return ViewRegistry;
}

start();
