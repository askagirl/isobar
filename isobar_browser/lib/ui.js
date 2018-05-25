import IsobarClient from "./client";

const server = new Worker("server.js");
const client = new IsobarClient(server);
const websocketAddress = "127.0.0.1:9999";
client.sendMessage({ type: "ConnectToWebsocket", address: websocketAddress });
client.onMessage(message => {});
