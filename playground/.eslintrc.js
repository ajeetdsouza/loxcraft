module.exports = {
  root: true,
  extends: [
    'airbnb',
    'plugin:@typescript-eslint/recommended',
    'plugin:import/recommended',
    'eslint:recommended',
    'plugin:@typescript-eslint/recommended-requiring-type-checking',
    'prettier',
  ],
  parser: '@typescript-eslint/parser',
  parserOptions: {
    // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment
    tsconfigRootDir: __dirname,
    project: ['./tsconfig.eslint.json']
  },
  plugins: [
    'react',
    'react-hooks',
    'jsx-a11y',
    '@typescript-eslint',
    'eslint-plugin-tsdoc'
  ],
  rules: {
    'import/extensions': 0,
    '@typescript-eslint/no-use-before-define': [
      'error',
    ],
    'react/jsx-filename-extension': [0, { extensions: ['.jsx', '.tsx'] }],
    'no-unused-vars': ['error'],
    quotes: [
      'error',
      'single',
    ],
    'import/no-extraneous-dependencies': 0,
    'react-hooks/rules-of-hooks': 'error',
    'react-hooks/exhaustive-deps': 'warn',
    'import/order': [
      'error',
      {
        groups: ['builtin', 'external', 'internal'],
        pathGroups: [
          {
            pattern: 'react',
            group: 'external',
            position: 'before',
          },
        ],
        pathGroupsExcludedImportTypes: ['react'],
        alphabetize: {
          order: 'asc',
          caseInsensitive: true,
        },
      },
    ],
    'react/function-component-definition': [
      2,
      {
        namedComponents: ['arrow-function', 'function-declaration'],
        unnamedComponents: 'arrow-function',
      },
    ],
    'tsdoc/syntax': 'warn'
  },
  settings: {
    'import/resolver': {
      node: {
        extensions: ['.js', '.jsx', '.ts', '.tsx'],
      },
      caseSensitive: false,
    },
  },
};
