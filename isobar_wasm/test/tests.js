import assert from "assert";
import { isobar as isobarPromise, JsSink } from "../lib/main";

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
        assert.equal(message.length, 1);
        messages.push(message[0]);
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
      sender.send([i++]);
    }, 1);
  });
});
