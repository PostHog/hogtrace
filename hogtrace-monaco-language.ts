/**
 * Monaco Editor Language Definition for HogTrace (TypeScript)
 *
 * Usage:
 * ```typescript
 * import * as monaco from 'monaco-editor';
 * import { hogtraceLanguage } from './hogtrace-monaco-language';
 *
 * // Register the language
 * monaco.languages.register({ id: 'hogtrace' });
 * monaco.languages.setMonarchTokensProvider('hogtrace', hogtraceLanguage.language);
 * monaco.languages.setLanguageConfiguration('hogtrace', hogtraceLanguage.configuration);
 *
 * // Optional: Register custom theme
 * monaco.editor.defineTheme('hogtrace-theme', hogtraceLanguage.theme);
 *
 * // Create editor
 * const editor = monaco.editor.create(document.getElementById('container')!, {
 *   value: 'fn:myapp.test:entry\n{\n    capture(args);\n}',
 *   language: 'hogtrace',
 *   theme: 'hogtrace-theme'
 * });
 * ```
 */

import type * as monaco from 'monaco-editor';

export interface HogTraceLanguage {
  configuration: monaco.languages.LanguageConfiguration;
  language: monaco.languages.IMonarchLanguage;
  theme: monaco.editor.IStandaloneThemeData;
}

export const hogtraceLanguage: HogTraceLanguage = {
  // Language configuration for auto-closing, brackets, etc.
  configuration: {
    comments: {
      lineComment: '#',
      blockComment: ['/*', '*/']
    },
    brackets: [
      ['{', '}'],
      ['[', ']'],
      ['(', ')']
    ],
    autoClosingPairs: [
      { open: '{', close: '}' },
      { open: '[', close: ']' },
      { open: '(', close: ')' },
      { open: '"', close: '"' },
      { open: "'", close: "'" },
      { open: '/', close: '/', notIn: ['string', 'comment'] }
    ],
    surroundingPairs: [
      { open: '{', close: '}' },
      { open: '[', close: ']' },
      { open: '(', close: ')' },
      { open: '"', close: '"' },
      { open: "'", close: "'" },
      { open: '/', close: '/' }
    ],
    folding: {
      markers: {
        start: new RegExp('\\{'),
        end: new RegExp('\\}')
      }
    }
  },

  // Monarch tokenizer/syntax highlighter
  language: {
    defaultToken: '',
    tokenPostfix: '.hogtrace',

    // Keywords and operators
    keywords: [
      'entry', 'exit', 'capture', 'send', 'sample'
    ],

    providers: ['fn', 'py'],

    builtinFunctions: [
      'timestamp', 'rand', 'len', 'args', 'arg0', 'arg1', 'arg2',
      'arg3', 'arg4', 'arg5', 'retval'
    ],

    booleans: ['True', 'False'],

    nullKeyword: ['None'],

    operators: [
      '=', '>', '<', '!', '==', '<=', '>=', '!=',
      '&&', '||', '+', '-', '*', '/', '%', '.'
    ],

    // Regular expressions for matching tokens
    symbols: /[=><!~?:&|+\-*\/\^%]+/,
    escapes: /\\(?:[abfnrtv\\"']|x[0-9A-Fa-f]{1,4}|u[0-9A-Fa-f]{4}|U[0-9A-Fa-f]{8})/,
    digits: /\d+(_+\d+)*/,

    // The main tokenizer
    tokenizer: {
      root: [
        // Probe specification - provider
        [/\b(fn|py)(?=:)/, 'keyword.provider'],

        // Probe points
        [/\b(entry|exit)\b/, 'keyword.probepoint'],

        // Keywords
        [/\b(capture|send|sample)\b/, 'keyword'],

        // Request variables
        [/\$(?:req|request)\.[a-zA-Z_]\w*/, 'variable.request'],

        // Built-in functions and variables
        [/\b(timestamp|rand|len|args|arg\d+|retval)\b/, 'function.builtin'],

        // Booleans
        [/\b(True|False)\b/, 'constant.boolean'],

        // None
        [/\bNone\b/, 'constant.null'],

        // Identifiers (must come after keywords)
        [/[a-zA-Z_]\w*/, {
          cases: {
            '@keywords': 'keyword',
            '@builtinFunctions': 'function.builtin',
            '@booleans': 'constant.boolean',
            '@nullKeyword': 'constant.null',
            '@default': 'identifier'
          }
        }],

        // Whitespace
        { include: '@whitespace' },

        // Predicate delimiters (guard conditions)
        [/\/(?!\*)/, { token: 'delimiter.predicate', next: '@predicate' }],

        // Delimiters and operators
        [/[{}()\[\]]/, '@brackets'],
        [/[<>](?!@symbols)/, '@brackets'],
        [/@symbols/, {
          cases: {
            '@operators': 'operator',
            '@default': ''
          }
        }],

        // Numbers
        [/\d*\.\d+([eE][\-+]?\d+)?/, 'number.float'],
        [/\d+[eE][\-+]?\d+/, 'number.float'],
        [/@digits/, 'number'],

        // Delimiter: colon for probe spec
        [/:/, 'delimiter.colon'],

        // Strings
        [/"([^"\\]|\\.)*$/, 'string.invalid'],  // non-terminated string
        [/'([^'\\]|\\.)*$/, 'string.invalid'],  // non-terminated string
        [/"/, 'string', '@string_double'],
        [/'/, 'string', '@string_single'],

        // Wildcard
        [/\*/, 'constant.wildcard'],

        // Delimiters
        [/[;,.]/, 'delimiter'],
        [/%/, 'operator.percent']
      ],

      // Predicate context (between /.../)
      predicate: [
        [/\//, { token: 'delimiter.predicate', next: '@pop' }],
        { include: 'root' }
      ],

      // Comments
      whitespace: [
        [/[ \t\r\n]+/, ''],
        [/#.*$/, 'comment'],
        [/\/\*/, 'comment', '@comment'],
      ],

      comment: [
        [/[^\/*]+/, 'comment'],
        [/\*\//, 'comment', '@pop'],
        [/[\/*]/, 'comment']
      ],

      // String tokenization
      string_double: [
        [/[^\\"]+/, 'string'],
        [/@escapes/, 'string.escape'],
        [/\\./, 'string.escape.invalid'],
        [/"/, 'string', '@pop']
      ],

      string_single: [
        [/[^\\']+/, 'string'],
        [/@escapes/, 'string.escape'],
        [/\\./, 'string.escape.invalid'],
        [/'/, 'string', '@pop']
      ],
    }
  },

  // Custom theme for HogTrace
  theme: {
    base: 'vs-dark', // or 'vs' for light theme
    inherit: true,
    rules: [
      { token: 'keyword.provider', foreground: 'C586C0', fontStyle: 'bold' },
      { token: 'keyword.probepoint', foreground: '569CD6', fontStyle: 'bold' },
      { token: 'keyword', foreground: '569CD6' },
      { token: 'variable.request', foreground: '9CDCFE', fontStyle: 'italic' },
      { token: 'function.builtin', foreground: 'DCDCAA' },
      { token: 'constant.boolean', foreground: '569CD6' },
      { token: 'constant.null', foreground: '569CD6' },
      { token: 'constant.wildcard', foreground: 'D4D4D4', fontStyle: 'bold' },
      { token: 'string', foreground: 'CE9178' },
      { token: 'string.escape', foreground: 'D7BA7D' },
      { token: 'string.invalid', foreground: 'FF0000' },
      { token: 'number', foreground: 'B5CEA8' },
      { token: 'number.float', foreground: 'B5CEA8' },
      { token: 'comment', foreground: '6A9955', fontStyle: 'italic' },
      { token: 'delimiter.predicate', foreground: 'D4D4D4', fontStyle: 'bold' },
      { token: 'delimiter.colon', foreground: 'D4D4D4' },
      { token: 'operator', foreground: 'D4D4D4' },
      { token: 'operator.percent', foreground: 'D4D4D4' },
      { token: 'identifier', foreground: '9CDCFE' }
    ],
    colors: {
      'editor.foreground': '#D4D4D4',
      'editor.background': '#1E1E1E'
    }
  }
};
