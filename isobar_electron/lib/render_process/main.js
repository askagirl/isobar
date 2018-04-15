process.env.NODE_ENV = "production"

const fs = require("fs");
const QueryString = require("querystring");
const path = require("path");
const isobar = require("isobar");
const React = require("react");
const ReactDOM = require("react-dom");
const Styletron = require("styletron-client");
const { StyletronProvider } = require("styletron-react");
const IsobarClient = require('../shared/isobar_client');

const ThemeProvider = require("./theme_provider");
const TextEditor = require("./text_editor/text_editor");

const {socketPath, windowId} = QueryString.parse(window.location.serach);

const isobarClient = new IsobarClient();
isobarClient.start(socketPath).then(() => {
  console.log('started!!!');

  isobarClient.addMessageListener(message => {
    console.log("MESSAGE", message);
  });

  isobarClient.sendMessage({
    type: 'StartWindow',
    window_id: windowId
  });

  setInterval(() => {
    isobarClient.sendMessage({
      type: 'Action',
      view_id: 0,
      action: {
        type: 'ToggleFileFinder'
      }
    });
  }, 1000);
});

const $ = React.createElement;

const theme = {
  editor: {
    fontFamily: "Menlo",
    backgroundColor: "white",
    baseTextColor: "black",
    fontSize: 14,
    lineHeight: 1.5
  }
}

ReactDOM.render(
  $(
    StyletronProvider,
    { styletron: new Styletron() },
    $(ThemeProvider, { theme: theme }, $(TextEditor, {
      initialText: fs.readFileSync(path.join(__dirname, '../../node_modules/react/cjs/react.development.js'), 'utf8')
    }))
  ),
  document.getElementById("app")
);
