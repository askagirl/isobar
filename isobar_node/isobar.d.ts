declare namespace Isobar {
  class TextBuffer {
    constructor(replicaId: number);
    length: number;
    getText(): string;
    splice(start: number, count: number, newText: string);
  }

  class TextEditor {
    constructor(buffer: TextBuffer, onChange: () => void);
    destroy(): void;
  }
}

declare module 'isobar' {
  export = Isobar;
}

interface NodeRequireFunction {
  (moduleName: 'isobar'): typeof Isobar;
}
