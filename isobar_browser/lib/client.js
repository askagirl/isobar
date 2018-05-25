export default class IsobarClient {
  constructor(worker) {
    this.worker = worker;
  }

  onMessage(callback) {
    this.worker.addEventListener("message", message => {
      callback(JSON.parse(message));
    });
  }

  sendMessage(message) {
    this.worker.postMessage(JSON.stringify(message));
  }
}
