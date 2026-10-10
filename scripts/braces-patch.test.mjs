import assert from 'node:assert/strict'
import { execFileSync } from 'node:child_process'
import { createRequire } from 'node:module'
import { test } from 'node:test'

// Resolve the installed copy used by shadcn's globbing dependency chain.
const frontendRequire = createRequire(
  new URL('../packages/frontend/package.json', import.meta.url)
)
const shadcnRequire = createRequire(
  frontendRequire.resolve('shadcn/tailwind.css')
)
const globRequire = createRequire(shadcnRequire.resolve('fast-glob'))
const micromatchRequire = createRequire(globRequire.resolve('micromatch'))
const bracesPath = micromatchRequire.resolve('braces')
const braces = micromatchRequire('braces')

const depthError = (error) =>
  error instanceof SyntaxError && /nesting depth exceeds/.test(error.message)

for (const [open, close] of [
  ['{', '}'],
  ['(', ')'],
]) {
  for (const operation of [
    'parse',
    'compile',
    'expand',
    'stringify',
    'default',
  ]) {
    test(`${operation} rejects deeply nested ${open}${close} with a bounded stack`, () => {
      const source = `
        const assert = require('node:assert/strict');
        const braces = require(${JSON.stringify(bracesPath)});
        const pattern = ${JSON.stringify(open)}.repeat(4000)
          + 'x' + ${JSON.stringify(close)}.repeat(4000);
        const operation = ${JSON.stringify(operation)};
        assert.throws(
          () => operation === 'default' ? braces(pattern) : braces[operation](pattern),
          error => error instanceof SyntaxError && /nesting depth exceeds/.test(error.message),
        );
      `
      execFileSync(process.execPath, ['--stack-size=512', '-e', source], {
        timeout: 5000,
        stdio: 'pipe',
      })
    })
  }
}

function nestedAst(depth) {
  let ast = { type: 'root', nodes: [] }
  for (let i = 0; i < depth; i++) {
    ast = { type: 'root', nodes: [ast] }
  }
  return ast
}

for (const operation of ['compile', 'expand', 'stringify']) {
  test(`${operation} bounds caller-supplied ASTs`, () => {
    assert.throws(() => braces[operation](nestedAst(4000)), depthError)
    assert.deepEqual(
      braces[operation](nestedAst(100)),
      operation === 'expand' ? [] : ''
    )
    assert.throws(() => braces[operation](nestedAst(101)), depthError)
  })
}

test('the parser accepts 100 containers and rejects 101', () => {
  for (const [open, close] of [
    ['{', '}'],
    ['(', ')'],
  ]) {
    assert.doesNotThrow(() =>
      braces.parse(open.repeat(100) + 'x' + close.repeat(100))
    )
    assert.throws(
      () => braces.parse(open.repeat(101) + 'x' + close.repeat(101)),
      depthError
    )
  }
})

test('ordinary glob patterns, ranges, escapes and parsed ASTs still work', () => {
  assert.equal(
    braces.compile('app/{reading,writing}/**/*.{js,jsx}'),
    'app/(reading|writing)/**/*.(js|jsx)'
  )
  assert.deepEqual(braces.expand('page-{1..3}.js'), [
    'page-1.js',
    'page-2.js',
    'page-3.js',
  ])
  assert.deepEqual(braces.expand('a\\{b,c\\}'), ['a{b,c}'])
  assert.equal(braces.compile(braces.parse('a/{b,c}/d')), 'a/(b|c)/d')
  const nested = '{'.repeat(99) + 'x' + '}'.repeat(99)
  assert.equal(braces.stringify(nested), nested)
  assert.deepEqual(braces.expand(nested), [nested])
})
