export let nanoPromise = import("../dist/nano_wasm");

async function load() {
  let {WorkTree, FileType} = await nanoPromise;
  let tree1 = WorkTree.new(BigInt(1));
  tree1.append_base_entry(1, "asd", FileType.Directory);
  tree1.append_base_entry(2, "foo", FileType.Directory);
  tree1.append_base_entry(3, "bar", FileType.Text);
  tree1.flush_base_entries();

  let tree2 = WorkTree.new(BigInt(1));
  tree2.append_base_entry(1, "asd", FileType.Directory);
  tree2.append_base_entry(2, "foo", FileType.Directory);
  tree2.append_base_entry(3, "bar", FileType.Text);
  let {file_id, operation} = tree2.next_text_file();


}

load();
