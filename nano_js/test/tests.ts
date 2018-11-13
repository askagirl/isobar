import * as nano from "../src/index";
import * as assert from "assert";

suite("WorkTree", () => {
  let WorkTree: typeof nano.WorkTree;

  suiteSetup(async () => {
    ({ WorkTree } = await nano.init());
  });

  test("basic API interaction", async () => {
    const OID_0 = "0".repeat(40);
    const OID_1 = "1".repeat(40);

    const git = new TestGitProvider();
    git.commit(OID_0, [
      { depth: 1, name: "a", type: nano.FileType.Directory },
      { depth: 2, name: "b", type: nano.FileType.Directory },
      { depth: 3, name: "c", type: nano.FileType.Text, text: "oid0 base text" },
      { depth: 3, name: "d", type: nano.FileType.Directory }
    ]);
    git.commit(OID_1, [
      { depth: 1, name: "a", type: nano.FileType.Directory },
      { depth: 2, name: "b", type: nano.FileType.Directory },
      { depth: 3, name: "c", type: nano.FileType.Text, text: "oid1 base text" }
    ]);

    const [tree1, initOps1] = WorkTree.create(OID_0, [], git);
    const [tree2, initOps2] = WorkTree.create(
      OID_0,
      await collectOps(initOps1),
      git
    );
    assert.strictEqual((await collectOps(initOps2)).length, 0);

    const ops1 = [];
    const ops2 = [];
    ops1.push(tree1.createFile("e", nano.FileType.Text).operation());
    ops2.push(tree2.createFile("f", nano.FileType.Text).operation());

    await assert.rejects(() => tree2.openTextFile("e"));

    ops1.push(...(await collectOps(tree1.applyOps(ops2.splice(0, Infinity)))));
    ops2.push(...(await collectOps(tree2.applyOps(ops1.splice(0, Infinity)))));
    assert.strictEqual(ops1.length, 0);
    assert.strictEqual(ops2.length, 0);

    const tree1BufferC = await tree1.openTextFile("a/b/c");
    assert.strictEqual(tree1BufferC.getPath(), "a/b/c");
    assert.strictEqual(tree1BufferC.getText(), "oid0 base text");
    const tree2BufferC = await tree2.openTextFile("a/b/c");
    assert.strictEqual(tree2BufferC.getPath(), "a/b/c");
    assert.strictEqual(tree2BufferC.getText(), "oid0 base text");

    const tree1BufferChanges: nano.Change[] = [];
    tree1BufferC.onChange(c => tree1BufferChanges.push(...c));
    ops1.push(
      tree1BufferC
        .edit(
          [
            { start: point(0, 4), end: point(0, 5) },
            { start: point(0, 9), end: point(0, 10) }
          ],
          "-"
        )
        .operation()
    );
    assert.strictEqual(tree1BufferC.getText(), "oid0-base-text");

    const tree2BufferChanges: nano.Change[] = [];
    tree2BufferC.onChange(c => tree2BufferChanges.push(...c));
    assert.deepStrictEqual(await collectOps(tree2.applyOps(ops1)), []);
    assert.strictEqual(tree1BufferC.getText(), "oid0-base-text");
    assert.deepStrictEqual(tree1BufferChanges, []);
    assert.deepStrictEqual(tree2BufferChanges, [
      { start: point(0, 4), end: point(0, 5), text: "-" },
      { start: point(0, 9), end: point(0, 10), text: "-" }
    ]);
    ops1.length = 0;

    ops1.push(tree1.createFile("x", nano.FileType.Directory).operation());
    ops1.push(tree1.createFile("x/y", nano.FileType.Directory).operation());
    ops1.push(tree1.rename("x", "a/b/x").operation());
    ops1.push(tree1.remove("a/b/d").operation());
    assert.deepStrictEqual(await collectOps(tree2.applyOps(ops1)), []);
    assert.deepStrictEqual(await collectOps(tree1.applyOps(ops2)), []);
    ops1.length = 0;
    ops2.length = 0;

    assert.deepStrictEqual(tree1.entries(), tree2.entries());
    assert.deepEqual(tree1.entries({ descendInto: [] }), [
      {
        depth: 1,
        type: nano.FileType.Directory,
        name: "a",
        path: "a",
        status: nano.FileStatus.Unchanged,
        visible: true
      },
      {
        depth: 1,
        type: nano.FileType.Text,
        name: "e",
        path: "e",
        status: nano.FileStatus.New,
        visible: true
      },
      {
        depth: 1,
        type: nano.FileType.Text,
        name: "f",
        path: "f",
        status: nano.FileStatus.New,
        visible: true
      }
    ]);
    assert.deepEqual(
      tree1.entries({ showDeleted: true, descendInto: ["a", "a/b"] }),
      [
        {
          depth: 1,
          type: nano.FileType.Directory,
          name: "a",
          path: "a",
          status: nano.FileStatus.Unchanged,
          visible: true
        },
        {
          depth: 2,
          type: nano.FileType.Directory,
          name: "b",
          path: "a/b",
          status: nano.FileStatus.Unchanged,
          visible: true
        },
        {
          depth: 3,
          type: nano.FileType.Text,
          name: "c",
          path: "a/b/c",
          status: nano.FileStatus.Modified,
          visible: true
        },
        {
          depth: 3,
          type: nano.FileType.Directory,
          name: "d",
          path: "a/b/d",
          status: nano.FileStatus.Removed,
          visible: false
        },
        {
          depth: 3,
          type: nano.FileType.Directory,
          name: "x",
          path: "a/b/x",
          status: nano.FileStatus.New,
          visible: true
        },
        {
          depth: 1,
          type: nano.FileType.Text,
          name: "e",
          path: "e",
          status: nano.FileStatus.New,
          visible: true
        },
        {
          depth: 1,
          type: nano.FileType.Text,
          name: "f",
          path: "f",
          status: nano.FileStatus.New,
          visible: true
        }
      ]
    );
    assert(tree1.exists("a/b/x"));
    assert(!tree1.exists("a/b/d"));

    tree1BufferChanges.length = 0;
    tree2BufferChanges.length = 0;
    ops1.push(...(await collectOps(tree1.reset(OID_1))));
    assert.deepStrictEqual(await collect(tree2.applyOps(ops1)), []);
    assert.strictEqual(tree1BufferC.getText(), "oid1 base text");
    assert.strictEqual(tree2BufferC.getText(), "oid1 base text");
    assert.deepStrictEqual(tree1BufferChanges, [
      { start: point(0, 3), end: point(0, 5), text: "1 " },
      { start: point(0, 9), end: point(0, 10), text: " " }
    ]);
    assert.deepStrictEqual(tree2BufferChanges, [
      { start: point(0, 3), end: point(0, 5), text: "1 " },
      { start: point(0, 9), end: point(0, 10), text: " " }
    ]);

    tree1.remove("a/b/c");
    assert(tree1BufferC.getPath() == null);
  });

  test("incomplete base oids", async () => {
    assert.throws(() => {
      const [tree, fixupOps] = WorkTree.create(
        "12345678",
        [],
        new TestGitProvider()
      );
    }, /123456789/);
  });

  test("buffer disposal", async () => {
    const OID = "0".repeat(40);
    const git = new TestGitProvider();
    git.commit(OID, [
      { depth: 1, name: "a", type: nano.FileType.Directory },
      { depth: 2, name: "b", type: nano.FileType.Directory },
      { depth: 3, name: "c", type: nano.FileType.Text, text: "oid0 base text" },
      { depth: 4, name: "d", type: nano.FileType.Directory }
    ]);

    const [tree1, initOps1] = WorkTree.create(OID, [], git);
    const [tree2, initOps2] = WorkTree.create(
      OID,
      await collectOps(initOps1),
      git
    );
    tree1.applyOps(await collectOps(initOps2));

    const buffer1 = await tree1.openTextFile("a/b/c");
    let buffer1ChangeCount = 0;
    buffer1.onChange(changes => buffer1ChangeCount++);

    const buffer2 = await tree2.openTextFile("a/b/c");
    tree1.applyOps([
      buffer2.edit([{ start: point(0, 0), end: point(0, 0) }], "x").operation()
    ]);
    assert.strictEqual(buffer1ChangeCount, 1);

    buffer1.dispose();
    tree1.applyOps([
      buffer2.edit([{ start: point(0, 0), end: point(0, 0) }], "y").operation()
    ]);
    assert.strictEqual(buffer1ChangeCount, 1);
  });
});

type BaseEntry =
  | nano.BaseEntry & { type: nano.FileType.Directory }
  | nano.BaseEntry & { type: nano.FileType.Text; text: string };

async function collect<T>(iterable: AsyncIterable<T>): Promise<T[]> {
  const items = [];
  for await (const item of iterable) {
    items.push(item);
  }
  return items;
}

async function collectOps(
  ops: ReadonlyArray<nano.OperationEnvelope>
): Promise<nano.Operation[]> {
  const envelopes = await collect(ops);
  return envelopes.map(envelope => envelope.operation());
}

function point(row: number, column: number): nano.Point {
  return { row, column };
}

class TestGitProvider implements nano.GitProvider {
  private entries: Map<nano.Oid, ReadonlyArray<nano.BaseEntry>>;
  private text: Map<nano.Oid, Map<nano.Path, string>>;

  constructor() {
    this.entries = new Map();
    this.text = new Map();
  }

  commit(oid: nano.Oid, entries: ReadonlyArray<BaseEntry>) {
    this.entries.set(oid, entries);

    const textByPath = new Map();
    const path = [];
    for (const entry of entries) {
      path.length = entry.depth - 1;
      path.push(entry.name);
      if (entry.type === nano.FileType.Text) {
        textByPath.set(path.join("/"), entry.text);
      }
    }
    this.text.set(oid, textByPath);
  }

  async *baseEntries(oid: nano.Oid): AsyncIterable<nano.BaseEntry> {
    const entries = this.entries.get(oid);
    if (entries) {
      for (const entry of entries) {
        yield entry;
      }
    } else {
      throw new Error("yy");
    }
  }

  async baseText(oid: nano.Oid, path: nano.Path): Promise<string> {
    const textByPath = this.text.get(oid);
    if (textByPath != null) {
      const text = textByPath.get(path);
      if (text != null) {
        await Promise.resolve();
        return text;
      } else {
        throw new Error(`No text found at path ${path}`);
      }
    } else {
      throw new Error(`No commit found with oid ${oid}`);
    }
  }
}
