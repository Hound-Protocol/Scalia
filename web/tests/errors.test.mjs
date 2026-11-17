import assert from 'node:assert/strict';
import test from 'node:test';
import { isFatalError } from '../errors.js';

test('GPU device failures are classified as fatal', () => {
  assert.equal(isFatalError(new Error('GPU_FATAL: device lost')), true);
  assert.equal(isFatalError(new Error('GPU_FATAL: out of memory')), true);
});

test('recoverable GPU errors do not trigger the crash path', () => {
  assert.equal(isFatalError(new Error('GPU_ERROR: surface validation failed')), false);
  assert.equal(isFatalError(new Error('GPU_ERROR: transient warning')), false);
});
