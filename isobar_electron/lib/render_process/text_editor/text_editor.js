const React = require("react");
const ReactDOM = require("react-dom");
const PropTypes = require("prop-types");
const { styled } = require("styletron-react");
const TextPlane = require('./text_plane');
const $ = React.createElement;

class TextEditorContainer extends React.Component {
  constructor(props) {
    super(props);
    this.onWheel = this.onWheel.bind(this);

    if (props.initialText) {
      buffer.splice(0, 0, props.initialText);
    }

    this.state = {
      resizeObserver: new ResizeObserver((entries) => this.componentDidResize(entries[0].cintentRect)),
      scrollTop: 0,
      height: 0,
      width: 0,
      showCursors: true
    };
  }

  componentDidMount() {
    const element = ReactDOM.findDOMNode(this);
    this.state.resizeObserver.observe(element);
    this.componentDidResize({
      width: element.offsetWidth,
      height: element.offsetHeight
    });

    element.addEventListener('wheel', this.onWheel, {passive: true});

    this.state.cursorBlinkIntervalHandle = window.setInterval(() => {
      this.setState({ showCursors: !this.state.showCursors });
    }, 500);
  }

  componentWillUnmount() {
    const element = ReactDOM.findDOMNode(this);
    element.removeEventListener('whell', this.onWheel, {passive: true});
    this.state.resizeObserver.disconnect();
    window.clearInterval(this.state.cursorBlinkIntervalHandle);
  }

  componentDidResize({width, height}) {
    this.setMeasurements({width, height})
  }

  setMeasurements(measurements) {
    this.setState(measurements)
    this.props.dispatch({type: 'SetMeasurements', measurements})
  }

  render() {
    const { scrollTop, width, height, showCursors } = this.state;

    return $(TextEditor, {
      scrollTop,
      width,
      height,
      showCursors,
      frameState: this.props
    });
  }

  onWheel (event) {
    this.setMeasurements({
      scrollTop: Math.max(0, this.state.scrollTop + event.deltaY)
    });
  }
}

TextEditorContainer.contextTypes = {
  theme: PropTypes.object
};

const Root = styled("div", {
  width: "100%",
  height: "100%",
  overflow: "hidden"
});

function TextEditor(props) {
  return $(
    Root,
    {onWhell: props.onWhell},
    $(TextPlane, {
      scrollTop: props.scrollTop,
      width: props.width,
      height: props.height,
      showCursors: props.showCursors,
      frameState: props.frameState
    })
  );
}

module.exports = TextEditorContainer;
