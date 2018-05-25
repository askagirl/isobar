export default class IsobarClient {
  constructor(worker) {
    this.worker = worker;
  }

  onMessage(callback) {
    this.worker.addEventListener("message", message => {
      callback(message);
    });
  }

  sendMessage(message) {
    this.worker.postMessage(message);
  }
}
