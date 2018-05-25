import assert from "assert";
import isobarPromise from "../lib/main"
import { JsSink } from "../lib/support"

suite("Server", () => {
  let isobar;

  before(async () => {
    isobar = await isobarPromise;
  });

  test("channels and sinks", endTest => {
    const test = isobar.Test.new();

    const messages = [];
    const sink = new JsSink({
      send(message) {
        messages.push(message);
      },

      close() {
        assert.deepEqual(messages, [0, 1, 2, 3, 4]);
        endTest();
      }
    });

    const channel = isobar.Channel.new();
    test.echo_stream(channel.take_receiver(), sink);

    const sender = channel.take_sender();
    let i = 0;
    let intervalId = setInterval(() => {
      if (i === 5) {
        sender.dispose();
        clearInterval(intervalId);
      }
      sender.send((i++).toString());
    }, 1);
  });
});
