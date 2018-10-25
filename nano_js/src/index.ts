import { BaseEntry, GitProvider, FileType, Oid } from './support';
import { GitProvider, GitProviderWrapper, Oid } from './support';
import { decode } from 'punycode'

let nano: any;

export async function init() {
  nano = await import("../dist/nano_js");
  nano.StreamToAsyncIterator.prototype[Symbol.asyncIterator] = function() {
      return this;
  }
  return { WorkTree };
}

type Tagged<BaseType, TagName> = BaseType & { __tag: TagName };

export type FileId = Tagged<string, "FileId">;
export type BufferId = Tagged<string, "BufferId">;
export type Version = Tagged<object, "Version">;
export type Operation = Tagged<string, "Operation">;

export class WorkTree {
  private tree: any;

  static create(replicaId: number, base: Oid, startOps: ReadonlyArray<Operation>, git: GitProvider): [WorkTree, AsyncIterable<Operation>] {
    const result = nano.WorkTree.new(new GitProviderWrapper(git), { replica_id: replicaId, base, startOps: startOps });
    return [new WorkTree(result.tree()), result.operations()];
  }

  constructor(tree: any) {
    this.tree = tree
  }

  newTextFile(): { fileId: FileId, operation: Operation } {
    const { file_id, operation } = this.tree.new_text_file();
    return { fileId: file_id, operation };
  }

  openTextFile(fileId: FileId): Promise<BufferId> {
    return this.tree.open_text_file(fileId);
  }
}
