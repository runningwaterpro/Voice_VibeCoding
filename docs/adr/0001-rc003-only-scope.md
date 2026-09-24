# RC003 is the only supported remote

The product scope is limited to the Xiaomi RC003. T1 and V60 are removed from the current product surface and implementation rather than kept as unfinished alternatives. This keeps reliability work focused and prevents users from seeing devices that cannot complete a real connection flow; any future device support must return as a separately scoped product decision.

The release must stop reading and writing T1/V60 runtime configuration. Existing user files are not silently deleted during migration; they may be removed only through an explicit, separately confirmed cleanup action.
