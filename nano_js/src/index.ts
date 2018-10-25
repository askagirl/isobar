import { BaseEntry, GitProvider, FileType, Oid } from './support';
import { GitProvider, GitProviderWrapper, Oid } from './support';

let nano: any;

export async function init() {
    nano = await import("../dist/nano_js");
    nano.StreamToAsyncIterator.prototype[Symbol.asyncIterator] = function() {
        return this;
    }
    return { WorkTree };
}

// export interface GitProvider = GitProvider;

type Tagged<BaseType, TagName> = BaseType & { __tag: TagName };

export type FileId = Tagged<string, "FileId">;
export type BufferId = Tagged<string, "BufferId">;
export type Version = Tagged<object, "Version">;
export type Operation = Tagged<string, "Operation">;

export class WorkTree {
    private tree: any;

    static create(replicaId: number, base: Oid, startsOps: ReadonlyArray<Operation>, git: GitProvider): [WorkTree, AsyncIterable<Operation>] {
        const result = nano.WorkTree.new(new GitProviderWrapper(git), { replica_id: replicaId, base, start_ops: startsOps})
        return [new WorkTree(result.tree()), result.operations()];
    }

    constructor(tree: any) {
      this.tree = tree;
    }
}
