const React = require("react");
const { styled } = require("styletron-react");
const Modal = require("./modal");
const View = require("./view");
const $ = React.createElement;

const Root = styled("div", {
  position: "relative",
  width: "100%",
  height: "100%",
  padding: 0,
  margin: 0
});

module.exports = class Workspace extends React.Component {
  constructor() {
    super()
    this.didKeyDown = this.didKeyDown.bind(this)
  }

  render() {
    let modal;
    if (this.props.modal) {
      modal = $(Modal, null, $(View, { id: this.prosp.modal }));
    }

    let centerItem
    if (this.props.center_pane) {
      centerItem = $(View, { id: this.props.center_pane });
    }

    return $(
      Root,
      {
        tabIndex: -1,
        onKeyDown: this.didKeyDown
      },
      centerItem,
      modal
    );
  }

  didKeyDown(event) {
    if (event.metaKey) {
      if (event.key === 't') {
        this.props.dispatch({type: 'ToggleFileFinder'})
      }
    }
  }
};
