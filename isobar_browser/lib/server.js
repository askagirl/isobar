import { isobar as isobarPromise, JsSink } from "isobar_wasm";

const serverPromise = isobarPromise.then(isobar => new Server(isobar));

global.addEventListener("message", handleMessage);

async function handleMessage(event) {
  const message = event.data;
  const server = await serverPromise;
  switch (message.type) {
    case "ConnectToWebsocket":
      server.connectToWebsocket(message.url);
      break;
    default:
      console.log("Received unknown message", message);
  }
}

class Server {
  constructor(isobar) {
    this.isobar = isobar;
    this.isobarServer = isobar.Server.new();
  }

  connectToWebsocket(url) {
    const socket = new WebSocket(url);
    socket.binaryType = "arraybuffer";
    const channel = this.isobar.Channel.new();
    const sender = channel.take_sender();
    const receiver = channel.take_receiver();

    console.log("connect", url);

    socket.addEventListener('message', function (event) {
      const data = new Uint8Array(event.data);
      console.log("received message", data);
      sender.send(data);
    });

    const sink = new JsSink({
      send(message) {
        console.log("send message", message);
        socket.send(message);
      },
    })

    this.isobarServer.connect_to_peer(receiver, sink)
  }
}
