#!/usr/bin/env bash
# Clones upstream solweig at the commit vendor/solweig/PROVENIENZA.toml pins,
# into banco/monte, and makes its crate linkable from a test binary: route A2
# of docs/ricerca/05-crate-rlib.md, `extension-module` removed from the pyo3
# features of the clone's manifest, so pyo3 links libpython.
#
# Exits non-zero when the clone or the manifest edit fails: a bench that skips
# itself would report green without comparing anything.
set -euo pipefail

banco=$(cd "$(dirname "$0")" && pwd)
provenienza="$banco/../vendor/solweig/PROVENIENZA.toml"
leggi() { grep -m1 "^$1 *=" "$provenienza" | sed 's/.*= *"\(.*\)"/\1/'; }
upstream=$(leggi upstream)
commit=$(leggi commit)
monte="$banco/monte"

if [ ! -d "$monte/.git" ]; then
  git clone --filter=blob:none --quiet "https://github.com/$upstream" "$monte"
fi
# --force puts back the manifest a previous run edited.
git -C "$monte" checkout --quiet --force "$commit"
test "$(git -C "$monte" rev-parse HEAD)" = "$commit"

manifesto="$monte/rust/Cargo.toml"
riga='pyo3 = { version = "0.24.2", features = ["extension-module", "abi3-py311"] }'
if [ "$(grep -cxF "$riga" "$manifesto")" != 1 ]; then
  echo "prepara.sh: $manifesto no longer carries the pyo3 line route A2 edits:" >&2
  echo "  $riga" >&2
  exit 1
fi
sed -i.orig 's/"extension-module", //' "$manifesto"
rm "$manifesto.orig"

echo "monte: $upstream @ $commit, extension-module removed"
# The interpreter pyo3-build-config 0.24.2 settles on, in its order
# (`find_interpreter` in its src/impl_.rs): PYO3_PYTHON, then the first of
# `python` and `python3` that answers "Python 3". An active virtualenv or conda
# environment would come before PATH; this script does not look for one.
python=${PYO3_PYTHON:-}
if [ -z "$python" ]; then
  for candidato in python python3; do
    if "$candidato" --version 2>&1 | grep -q '^Python 3'; then
      python=$(command -v "$candidato")
      break
    fi
  done
fi
if [ -z "$python" ]; then
  echo "prepara.sh: no Python 3 for pyo3 to link against" >&2
  exit 1
fi
echo "python: $python, $("$python" --version 2>&1)"
"$python" -c 'import sysconfig as s; print("python: LIBDIR", s.get_config_var("LIBDIR"), "Py_ENABLE_SHARED", s.get_config_var("Py_ENABLE_SHARED"), "framework", s.get_config_var("PYTHONFRAMEWORK") or "-")'
