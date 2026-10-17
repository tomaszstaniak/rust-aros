# Runtime validator review fixes — 2026-10-17

Two P2 review findings were reproduced and fixed in the host tools. Native
programs, private runtime patches, Photocraft and SDK remain unchanged. No VM
was started or controlled in this follow-up.

## Contradictory baseline exit evidence

Previously, the checker accepted observer timeout plus diagnostic log even if
the Shell had written the baseline rc file. A lost network end receipt could
therefore turn a completed process into an accepted expected hang. The checker
now requires the baseline rc path to be absent (including dangling symlinks)
and absent from the collection SHA manifest. Any rc content contradicts the
claim of no Shell return, including zero, nonzero, empty or malformed content.
A manifest entry also rejects evidence when its file has disappeared.

Full-checker tests use real temporary report files/manifests. They observed14
negative subcase failures before the fix, then passed: all three baseline cases
with four rc contents, plus unmanifested rc and manifest-only rc. A valid
baseline with no rc files continues to pass. Parsing the code or merely checking
its digest would not resolve the contradiction; presence itself is decisive.

## Absolute receive/acknowledgment deadline

Previously each recv reused the original socket timeout, so small successive
writes extended total processing. A real loopback peer sending one byte every
30ms kept the0.1s observer running about0.633s before the fix. The regression
failed its0.35s scheduler-tolerant upper bound. With the fix it passes and no
late OK acknowledgment is received. Elapsed assertion measures collect itself,
not thread join/cleanup time.

Each accepted peer now gets a fixed absolute deadline: the earlier of the
nearest active case deadline and the existing1s peer cap. Remaining monotonic
time is recomputed before every recv and before sendall of OK. Exhausted
budgets raise timeout; no positive minimum extends an expired budget. Listener
accept similarly uses actual remaining time. Ordinary complete, stale and
out-of-order receipt tests retain their behavior.

## Verification

Full host suite71/71 passed. The corrected full checker revalidated the original
bundle02 run e41ec53f-413d-4b91-9818-92f453b0e24c from retained private build,
results-before/results-after and original observations. It passed; new
revalidation.json is separate from the historical validation. The historical
build inputs, logs and collection manifests were not edited. The new validator
source/test digests are in tools.json. Original native evidence remains in
../2026-10-17-runtime; this follow-up does not claim a native run with the
changed observer. Real host TCP tests exercise the changed observer behavior.
