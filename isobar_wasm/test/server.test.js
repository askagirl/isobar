import assert from "assert";
import isobarPromise from "../lib/index"
import { JsSender } from "../lib/support"

suite("Server", () => {
  let isobar = null;

  before(async () => {
    isobar = await isobarPromise;
  })

  test("smoke test", finish => {
    const pair = isobar.ChannelPair.new();
    const test = isobar.Test.new();
    const outgoing = new JsSender();
    const messages = [];
    outgoing.onMessage = m => messages.push(parseInt(m));
    outgoing.onFinish = () => {
      assert.deepEqual(messages, [0, 1, 2, 3, 4]);
      finish();
    };
    test.echo_stream(outgoing, pair.tx());

    const tx = pair.tx();
    let i = 0;
    let intervalId = setInterval(() => {
      if (i === 5) {
        tx.dispose();
        clearInterval(intervalId);
      }

      tx.send((i++).toString());
    }, 1);
  })
});
