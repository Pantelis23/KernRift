import {
    createConnection, TextDocuments, ProposedFeatures,
    InitializeParams, InitializeResult, TextDocumentSyncKind,
    CompletionItem, CompletionItemKind, Hover, MarkupKind,
    Diagnostic, DiagnosticSeverity, DefinitionParams, Location,
    Range, Position
} from 'vscode-languageserver/node';
import { TextDocument } from 'vscode-languageserver-textdocument';
import { execSync } from 'child_process';
import * as path from 'path';

const connection = createConnection(ProposedFeatures.all);
const documents = new TextDocuments(TextDocument);

let krcPath = 'krc';

// --- Keywords and builtins ---

const KEYWORDS = [
    'fn', 'struct', 'enum', 'static', 'const', 'type', 'unsafe', 'import',
    'if', 'else', 'while', 'for', 'in', 'break', 'continue', 'return', 'match',
    'true', 'false', 'extern', 'export'
];

const TYPES = [
    'uint8', 'uint16', 'uint32', 'uint64',
    'int8', 'int16', 'int32', 'int64',
    'bool', 'char', 'u8', 'u16', 'u32', 'u64',
    'i8', 'i16', 'i32', 'i64', 'addr', 'byte'
];

const BUILTINS: Record<string, { sig: string, doc: string }> = {
    'exit': { sig: 'fn exit(code: uint64)', doc: 'Terminate the process with the given exit code.' },
    'write': { sig: 'fn write(fd: uint64, buf: uint64, len: uint64)', doc: 'Write `len` bytes from `buf` to file descriptor `fd`.' },
    'alloc': { sig: 'fn alloc(size: uint64) -> uint64', doc: 'Allocate `size` bytes of memory. Returns pointer to allocated block.' },
    'dealloc': { sig: 'fn dealloc(ptr: uint64)', doc: 'Free allocated memory (no-op on most platforms).' },
    'print': { sig: 'fn print(arg)', doc: 'Print a string literal or format an integer to stdout.' },
    'println': { sig: 'fn println(arg)', doc: 'Print a string literal or integer followed by a newline.' },
    'str_len': { sig: 'fn str_len(s: uint64) -> uint64', doc: 'Return the length of a null-terminated string.' },
    'str_eq': { sig: 'fn str_eq(a: uint64, b: uint64) -> uint64', doc: 'Compare two strings. Returns 1 if equal, 0 if not.' },
    'memcpy': { sig: 'fn memcpy(dst: uint64, src: uint64, len: uint64) -> uint64', doc: 'Copy `len` bytes from `src` to `dst`. Returns `dst`.' },
    'memset': { sig: 'fn memset(dst: uint64, val: uint64, len: uint64) -> uint64', doc: 'Fill `len` bytes at `dst` with `val`. Returns `dst`.' },
    'file_open': { sig: 'fn file_open(path: uint64, flags: uint64) -> uint64', doc: 'Open a file. flags: 0=read, 1=write. Returns file descriptor.' },
    'file_read': { sig: 'fn file_read(fd: uint64, buf: uint64, len: uint64)', doc: 'Read `len` bytes from `fd` into `buf`.' },
    'file_write': { sig: 'fn file_write(fd: uint64, buf: uint64, len: uint64)', doc: 'Write `len` bytes from `buf` to `fd`.' },
    'file_close': { sig: 'fn file_close(fd: uint64)', doc: 'Close a file descriptor.' },
    'file_size': { sig: 'fn file_size(fd: uint64) -> uint64', doc: 'Return the size of the file in bytes.' },
};

// --- Initialize ---

connection.onInitialize((params: InitializeParams): InitializeResult => {
    const config = params.initializationOptions;
    if (config?.compilerPath) krcPath = config.compilerPath;

    return {
        capabilities: {
            textDocumentSync: TextDocumentSyncKind.Full,
            completionProvider: { triggerCharacters: ['.', '@', '"'] },
            hoverProvider: true,
            definitionProvider: true,
        }
    };
});

// --- Diagnostics via krc check ---

function validateDocument(doc: TextDocument): void {
    const text = doc.getText();
    const uri = doc.uri;
    const filePath = uri.replace('file://', '');
    const diagnostics: Diagnostic[] = [];

    // Parse error patterns: "error at line N: message"
    const errorRegex = /error at line (\d+): (.+)/g;
    // Also check for common issues ourselves
    const lines = text.split('\n');

    // Try running krc check if available
    try {
        const tmpFile = `/tmp/krc_lsp_${process.pid}.kr`;
        require('fs').writeFileSync(tmpFile, text);
        const result = execSync(`${krcPath} check ${tmpFile} 2>&1`, {
            timeout: 5000,
            encoding: 'utf8'
        });

        let match;
        while ((match = errorRegex.exec(result)) !== null) {
            const line = parseInt(match[1]) - 1;
            const message = match[2];
            diagnostics.push({
                severity: DiagnosticSeverity.Error,
                range: {
                    start: { line, character: 0 },
                    end: { line, character: lines[line]?.length || 0 }
                },
                message,
                source: 'krc'
            });
        }

        require('fs').unlinkSync(tmpFile);
    } catch (e: any) {
        // Parse compiler error output
        const output = e.stdout || e.stderr || e.message || '';
        let match;
        while ((match = errorRegex.exec(output)) !== null) {
            const line = parseInt(match[1]) - 1;
            const message = match[2];
            if (line >= 0 && line < lines.length) {
                diagnostics.push({
                    severity: DiagnosticSeverity.Error,
                    range: {
                        start: { line, character: 0 },
                        end: { line, character: lines[line]?.length || 0 }
                    },
                    message,
                    source: 'krc'
                });
            }
        }
    }

    // Quick local checks
    for (let i = 0; i < lines.length; i++) {
        const line = lines[i].trim();
        // Check for unclosed braces (simple heuristic)
        if (line.match(/^\s*fn\s+/) && !line.includes('{') && !line.includes(';') && !lines[i + 1]?.trim().startsWith('{')) {
            // Might be missing opening brace — skip, krc check handles this
        }
    }

    connection.sendDiagnostics({ uri, diagnostics });
}

documents.onDidChangeContent(change => {
    validateDocument(change.document);
});

documents.onDidSave(change => {
    validateDocument(change.document);
});

// --- Completions ---

connection.onCompletion((params): CompletionItem[] => {
    const doc = documents.get(params.textDocument.uri);
    if (!doc) return [];

    const text = doc.getText();
    const offset = doc.offsetAt(params.position);
    const lineText = text.substring(text.lastIndexOf('\n', offset - 1) + 1, offset);
    const items: CompletionItem[] = [];

    // After @: suggest annotations
    if (lineText.endsWith('@')) {
        ['export', 'ctx', 'eff', 'caps'].forEach((name, i) => {
            items.push({
                label: name,
                kind: CompletionItemKind.Keyword,
                detail: `@${name}`,
                sortText: String(i)
            });
        });
        return items;
    }

    // After import ": suggest std/ modules
    if (lineText.match(/import\s+"$/)) {
        ['std/string.kr', 'std/io.kr', 'std/math.kr', 'std/fmt.kr',
         'std/mem.kr', 'std/vec.kr', 'std/map.kr'].forEach((mod, i) => {
            items.push({
                label: mod,
                kind: CompletionItemKind.Module,
                insertText: mod,
                sortText: String(i)
            });
        });
        return items;
    }

    // After dot: suggest struct fields (basic — scan for struct definitions)
    if (lineText.endsWith('.')) {
        const word = lineText.slice(0, -1).match(/(\w+)$/)?.[1];
        if (word) {
            // Find struct type of this variable
            const varDecl = text.match(new RegExp(`(\\w+)\\s+${word}\\b`));
            if (varDecl) {
                const structName = varDecl[1];
                const structMatch = text.match(new RegExp(`struct\\s+${structName}\\s*\\{([^}]*)\\}`));
                if (structMatch) {
                    const fields = structMatch[1].match(/\w+\s+(\w+)/g);
                    fields?.forEach(f => {
                        const fieldName = f.split(/\s+/)[1];
                        if (fieldName) {
                            items.push({
                                label: fieldName,
                                kind: CompletionItemKind.Field
                            });
                        }
                    });
                }
            }
        }
        return items;
    }

    // General completions: keywords, types, builtins, functions in file
    KEYWORDS.forEach(kw => {
        items.push({ label: kw, kind: CompletionItemKind.Keyword });
    });

    TYPES.forEach(t => {
        items.push({ label: t, kind: CompletionItemKind.TypeParameter });
    });

    Object.keys(BUILTINS).forEach(name => {
        items.push({
            label: name,
            kind: CompletionItemKind.Function,
            detail: BUILTINS[name].sig,
            documentation: BUILTINS[name].doc
        });
    });

    // Functions defined in the file
    const fnRegex = /fn\s+(\w+)\s*\(/g;
    let match;
    while ((match = fnRegex.exec(text)) !== null) {
        const name = match[1];
        if (!BUILTINS[name] && name !== 'main') {
            items.push({
                label: name,
                kind: CompletionItemKind.Function,
                detail: `fn ${name}(...)`,
            });
        }
    }

    // Structs defined in the file
    const structRegex = /struct\s+(\w+)/g;
    while ((match = structRegex.exec(text)) !== null) {
        items.push({
            label: match[1],
            kind: CompletionItemKind.Struct,
        });
    }

    // Enums defined in the file
    const enumRegex = /enum\s+(\w+)/g;
    while ((match = enumRegex.exec(text)) !== null) {
        items.push({
            label: match[1],
            kind: CompletionItemKind.Enum,
        });
    }

    return items;
});

// --- Hover ---

connection.onHover((params): Hover | null => {
    const doc = documents.get(params.textDocument.uri);
    if (!doc) return null;

    const text = doc.getText();
    const offset = doc.offsetAt(params.position);

    // Extract word at cursor
    const before = text.substring(0, offset);
    const after = text.substring(offset);
    const wordStart = before.match(/(\w+)$/)?.[1] || '';
    const wordEnd = after.match(/^(\w+)/)?.[1] || '';
    const word = wordStart + wordEnd;

    if (!word) return null;

    // Check builtins
    if (BUILTINS[word]) {
        return {
            contents: {
                kind: MarkupKind.Markdown,
                value: `**${word}**\n\n\`\`\`kr\n${BUILTINS[word].sig}\n\`\`\`\n\n${BUILTINS[word].doc}`
            }
        };
    }

    // Check keywords
    if (KEYWORDS.includes(word)) {
        const kwDocs: Record<string, string> = {
            'fn': 'Declare a function.',
            'struct': 'Define a struct type.',
            'enum': 'Define an enumeration.',
            'match': 'Match an expression against multiple arms.',
            'import': 'Import another KernRift source file.',
            'type': 'Create a type alias.',
            'static': 'Declare a static (global) variable.',
            'unsafe': 'Mark a block for raw pointer operations.',
            'if': 'Conditional branch.',
            'while': 'Loop while condition is true.',
            'for': 'Iterate over a range: `for i in start..end`.',
            'return': 'Return a value from a function.',
            'break': 'Exit a loop.',
            'continue': 'Skip to next loop iteration.',
        };
        if (kwDocs[word]) {
            return { contents: { kind: MarkupKind.Markdown, value: `**${word}** — ${kwDocs[word]}` } };
        }
    }

    // Check types
    if (TYPES.includes(word)) {
        const sizes: Record<string, string> = {
            'uint8': '8-bit unsigned (1 byte)', 'uint16': '16-bit unsigned (2 bytes)',
            'uint32': '32-bit unsigned (4 bytes)', 'uint64': '64-bit unsigned (8 bytes)',
            'int8': '8-bit signed', 'int16': '16-bit signed',
            'int32': '32-bit signed', 'int64': '64-bit signed',
            'bool': 'Boolean (true/false)', 'char': 'ASCII character (1 byte)',
        };
        return { contents: { kind: MarkupKind.Markdown, value: `**${word}** — ${sizes[word] || 'Type alias'}` } };
    }

    // Check functions in file
    const fnMatch = text.match(new RegExp(`fn\\s+${word}\\s*\\(([^)]*)\\)(?:\\s*->\\s*(\\w+))?`));
    if (fnMatch) {
        const params = fnMatch[1];
        const ret = fnMatch[2] ? ` -> ${fnMatch[2]}` : '';
        return {
            contents: {
                kind: MarkupKind.Markdown,
                value: `\`\`\`kr\nfn ${word}(${params})${ret}\n\`\`\``
            }
        };
    }

    return null;
});

// --- Go to Definition ---

connection.onDefinition((params: DefinitionParams): Location | null => {
    const doc = documents.get(params.textDocument.uri);
    if (!doc) return null;

    const text = doc.getText();
    const offset = doc.offsetAt(params.position);

    const before = text.substring(0, offset);
    const after = text.substring(offset);
    const wordStart = before.match(/(\w+)$/)?.[1] || '';
    const wordEnd = after.match(/^(\w+)/)?.[1] || '';
    const word = wordStart + wordEnd;

    if (!word) return null;

    // Search for function definition
    const fnRegex = new RegExp(`fn\\s+${word}\\s*\\(`, 'g');
    const match = fnRegex.exec(text);
    if (match) {
        const pos = doc.positionAt(match.index);
        return Location.create(params.textDocument.uri, Range.create(pos, pos));
    }

    // Search for struct definition
    const structRegex = new RegExp(`struct\\s+${word}\\s*\\{`, 'g');
    const sMatch = structRegex.exec(text);
    if (sMatch) {
        const pos = doc.positionAt(sMatch.index);
        return Location.create(params.textDocument.uri, Range.create(pos, pos));
    }

    // Search for enum definition
    const enumRegex = new RegExp(`enum\\s+${word}\\s*\\{`, 'g');
    const eMatch = enumRegex.exec(text);
    if (eMatch) {
        const pos = doc.positionAt(eMatch.index);
        return Location.create(params.textDocument.uri, Range.create(pos, pos));
    }

    return null;
});

documents.listen(connection);
connection.listen();
