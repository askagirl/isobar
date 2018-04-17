process.env.NODE_ENV = "production";

const App = require("./app");
const FileFinderComponent = require("./file_finder_component");
const QueryString = require("querystring");
const React = require("react");
const ReactDOM = require("react-dom");
const ViewRegistry = require("./view_registry");
const WorkspaceComponent = require("./workspace_component");
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

  isobarClient.sendMessage({ type: "StartWindow", window_id: Number(windowId) });
}

function buildViewRegistry(client) {
  const viewRegistry = new ViewRegistry({
    onAction: action => {
      action.type = "Action";
      client.sendMessage(action);
    }
  });
  viewRegistry.addComponent("Workspace", WorkspaceComponent);
  viewRegistry.addComponent("FileFinderView", FileFinderComponent);
  return ViewRegistry;
}

start();
