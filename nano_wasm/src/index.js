export let nanoPromise = import("../dist/nano_wasm");

export async function initialize() {
  const nano = nanoPromise;

  class WorkTree {
    constructor(replicaId) {
      this.workTree = nano.WorkTree.new(BigInt(replicaId));
    }

    appendBaseEntries(baseEntries) {
      for (const baseEntry of baseEntries) {
        this.workTree.push_base_entry(
          baseEntry.depth,
          baseEntry.name,
          baseEntry.type
        );
      }
      return collect(this.workTree.flush_base_entries());
    }

    applyOps(ops) {
      for (const op of ops) {
        this.workTree.push_op(op);
      }
      return collect(this.workTree.flush_ops());
    }

    newTextFile() {
      let result = this.workTree.new_text_file();
      return { fileId: result.file_id(), operation: result.operation() };
    }

    newDirectory(parentId, name) {
      let result = this.workTree.new_directory(parentId, name);
      return { fileId: result.file_id(), operation: result.operation() };
    }

    openTextFile(fileId, baseText) {
      let result = this.workTree.open_text_file(fileId, baseText);
      if (result.is_ok()) {
        return result.buffer_id();
      } else {
        throw new Error(result.error().to_string());
      }
    }
  }

  return { WorkTree, FileType: nano.FileType };
}

function collect(iterator) {
  let items = [];
  while (iterator.has_next()) {
    items.push(iterator.next());
  }
  return items;
}
