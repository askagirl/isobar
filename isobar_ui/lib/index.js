const FileFinder = require("./file_finder");
const ViewRegistry = require("./view_registry");
const Workspace = require("./workspace")
const TextEditorView = require("./text_editor/text_editor");
const Discussion = require("./discussion")

exports.buildViewRegistry = function buildViewRegistry(client) {
  const ViewRegistry = new ViewRegistry({
    onAction: action => {
      action.type = "Action";
      client.sendMessage(action);
    }
  });
  ViewRegistry.addComponent("Workspace", Workspace);
  ViewRegistry.addComponent("FileFinder", FileFinder);
  ViewRegistry.addComponent("BufferView", TextEditorView);
  ViewRegistry.addComponent("Discussion", Discussion);
  return ViewRegistry;
};

exports.App = require("./app");
exports.React = require("react");
exports.ReactDOM = require("react-dom");
