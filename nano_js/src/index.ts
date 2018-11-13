export {
  BaseEntry,
  Change,
  GitProvider,
  FileType,
  Oid,
  Path,
  Point,
  Range
} from "./support";
import {
  BufferId,
  Change,
  ChangeObserver,
  ChangeObserverCallback,
  CompositeDisposable,
  GitProvider,
  GitProviderWrapper,
  FileType,
  Oid,
  Path,
  Point,
  Range,
  Tagged
} from "./support";
import { randomBytes } from "crypto"

let nano: any;

export async function init() {
  nano = await import("../dist/nano_js");
  nano.StreamToAsyncIterator.prototype[Symbol.asyncIterator] = function() {
      return this;
  };
  return { WorkTree };
}

export type Version = Tagged<string, "Version">;
export type Operation = Tagged<Uint8Array, "Operation">;
export type ReplicaId = Tagged<string, "ReplicaId">;
export interface OperationEnvelope {
  epochTimestamp(): number;
  epochReplicaId(): ReplicaId;
  operation(): Operation;
}

export enum FileStatus {
  New = "New",
  Renamed = "Renamed",
  Removed = "Removed",
  Modified = "Modified",
  RenamedAndModified = "RenamedAndModified",
  Unchanged = "Unchanged"
}

export interface Entry {
  readonly depth: number;
  readonly type: FileType;
  readonly name: string;
  readonly path: string;
  readonly status: FileStatus;
  readonly visible: boolean;
}

export class WorkTree {
  private tree: any;
  private observer: ChangeObserver;

  static create(
    base: Oid | null,
    startOps: ReadonlyArray<Operation>,
    git: GitProvider
  ): [WorkTree, AsyncIterable<OperationEnvelope>] {
    const observer = new ChangeObserver();
    const result = nano.WorkTree.new(
      new GitProviderWrapper(git),
      observer,
      randomBytes(16),
      base,
      startOps
    );
    return [new WorkTree(result.tree(), observer), result.operations()];
  }

  private constructor(tree: any, observer: ChangeObserver) {
    this.tree = tree
    this.observer = observer;
  }

  reset(base: Oid | null): AsyncIterable<OperationEnvelope> {
    return this.tree.reset(base);
  }

  applyOps(ops: Operation[]): AsyncIterable<OperationEnvelope> {
    return this.tree.apply_ops(ops);
  }

  createFile(path: Path, fileType: FileType): OperationEnvelope {
    return this.tree.create_file(path, fileType);
  }

  rename(oldPath: Path, newPath: Path): OperationEnvelope {
    return this.tree.rename(oldPath, newPath);
  }

  remove(path: Path): OperationEnvelope {
    return this.tree.remove(path);
  }

  exists(path: Path): boolean {
    return this.tree.exists(path);
  }

  entries(options?: { descendInto?: Path[]; showDeleted?: boolean }): Entry[] {
    let descendInto = null;
    let showDeleted = false;
    if (options) {
      if (options.descendInto) descendInto = options.descendInto;
      if (options.showDeleted) showDeleted = options.showDeleted;
    }
    return this.tree.entries(descendInto, showDeleted);
  }

  async openTextFile(path: Path): Promise<Buffer> {
    const bufferId = await this.tree.open_text_file(path);
    return new Buffer(bufferId, this.tree, this.observer);
  }
}

export class Buffer {
  private id: BufferId;
  private tree: any;
  private observer: ChangeObserver;
  private disposables: CompositeDisposable;

  constructor(id: BufferId, tree: any, observer: ChangeObserver) {
    this.id = id;
    this.tree = tree;
    this.observer = observer;
    this.disposables = new CompositeDisposable();
  }

  dispose() {
    this.disposables.dispose();
  }

  edit(oldRanges: Range[], newText: string): OperationEnvelope {
    return this.tree.edit(this.id, oldRanges, newText);
  }

  getPath(): string | null {
    return this.tree.path(this.id);
  }

  getText(): string {
    return this.tree.text(this.id);
  }

  onChange(callback: ChangeObserverCallback) {
    this.disposables.add(this.observer.onChange(this.id, callback));
  }
}
