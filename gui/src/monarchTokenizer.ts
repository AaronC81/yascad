// To test changes: https://microsoft.github.io/monaco-editor/monarch.html
// Based off the "mylang" one

export default {
  keywords: [
    // Language-level keywords
    'operator', 'it', 'for',

    // Not really keywords, but important/special enough to highlight like one
    'buffer', 'children', 'copy',
  ],

  operators: ['=', '+', '-', '*', '/',],

  symbols:  /[=><!~?:&|+\-*\/\^%]+/,

  // The main tokenizer for our languages
  tokenizer: {
    root: [
      // identifiers and keywords
      [/[a-z_$][\w$]*/, { cases: { '@keywords': 'keyword', '@default': 'identifier' } }],

      // whitespace
      { include: '@whitespace' },

      // delimiters and operators
      [/[{}()\[\]]/, '@brackets'],
      [/[<>](?!@symbols)/, '@brackets'],
      [/@symbols/, { cases: { '@operators': 'operator',
                              '@default'  : '' } } ],

      // numbers
      [/\d*\.\d+([eE][\-+]?\d+)?/, 'number.float'],
      [/\d+/, 'number'],

      // delimiter: after number because of .\d floats
      [/[;,.]/, 'delimiter'],
    ],

    comment: [
      [/[^\/*]+/, 'comment' ],
      [/[\/*]/,   'comment' ]
    ],

    whitespace: [
      [/[ \t\r\n]+/, 'white'],
      [/\/\/.*$/,    'comment'],
    ],
  },
};