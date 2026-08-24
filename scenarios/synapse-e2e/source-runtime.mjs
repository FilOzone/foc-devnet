/**
 * Runs foc-devnet scenarios against Synapse TypeScript source on Node 24+.
 *
 * Source profiles install Synapse's production dependency closure, then preload
 * this module with `node --import`. Public `@filoz/synapse-sdk` and
 * `@filoz/synapse-core` imports resolve to their source counterparts instead of
 * the packages' compiled `dist` targets. All other imports use Node's normal
 * resolver.
 *
 * Mappings come from each package's export map, keeping the scenario on the
 * public API and avoiding a hard-coded list that drifts as exports change.
 * Wildcard exports are deliberately excluded because one wildcard can expose
 * paths with no TypeScript source equivalent.
 */
import assert from 'node:assert/strict'
import { existsSync, readFileSync } from 'node:fs'
import { registerHooks } from 'node:module'
import { dirname, join, resolve } from 'node:path'
import { pathToFileURL } from 'node:url'

const sourceRoot = process.env.SYNAPSE_SDK_SOURCE_DIR
assert(sourceRoot, 'SYNAPSE_SDK_SOURCE_DIR must name the Synapse checkout when using source-runtime.mjs')

// Export entries may be strings or nested condition objects. Prefer the
// conditions used by this ESM runtime, then inspect package-specific branches.
function exportedTarget(entry) {
  if (typeof entry === 'string') return entry
  if (entry == null || typeof entry !== 'object') return undefined
  for (const condition of ['node', 'import', 'default']) {
    const target = exportedTarget(entry[condition])
    if (target != null) return target
  }
  for (const target of Object.values(entry)) {
    const resolved = exportedTarget(target)
    if (resolved != null) return resolved
  }
  return undefined
}

// Convert concrete public dist exports to source files only when the matching
// TypeScript file exists. Dependencies and private paths remain untouched.
function sourceExports(packageName, packageDirectory) {
  const packagePath = join(sourceRoot, packageDirectory, 'package.json')
  const packageJson = JSON.parse(readFileSync(packagePath, 'utf8'))
  const mappings = new Map()
  for (const [subpath, entry] of Object.entries(packageJson.exports ?? {})) {
    if (subpath.includes('*')) continue
    const target = exportedTarget(entry)
    if (target == null) continue
    const sourcePath = resolve(
      dirname(packagePath),
      target.replace(/^\.\/dist\/src\//, './src/').replace(/\.js$/, '.ts')
    )
    if (!existsSync(sourcePath)) continue
    mappings.set(subpath === '.' ? packageName : `${packageName}/${subpath.slice(2)}`, pathToFileURL(sourcePath).href)
  }
  return mappings
}

const sourceMappings = new Map([
  ...sourceExports('@filoz/synapse-sdk', 'packages/synapse-sdk'),
  ...sourceExports('@filoz/synapse-core', 'packages/synapse-core'),
])

// Short-circuit exact public package matches; delegate everything else.
registerHooks({
  resolve(specifier, context, nextResolve) {
    const sourceUrl = sourceMappings.get(specifier)
    return sourceUrl == null ? nextResolve(specifier, context) : { url: sourceUrl, shortCircuit: true }
  },
})
