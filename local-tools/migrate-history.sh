#!/usr/bin/env bash
# Migrate Warp Stable local state (history DB, settings, themes) to the
# local warp-oss build. Run with BOTH Warp Stable and warp-oss quit.
set -euo pipefail

STABLE_DB_DIR="$HOME/Library/Group Containers/2BBY89MBSN.dev.warp/Library/Application Support/dev.warp.Warp-Stable"
# The dev build lands in the group container when it's writable (verified on
# first launch), otherwise plain Application Support.
OSS_DB_DIR="$HOME/Library/Group Containers/2BBY89MBSN.dev.warp/Library/Application Support/dev.warp.WarpOss"
if [ ! -d "$OSS_DB_DIR" ] && [ -d "$HOME/Library/Application Support/dev.warp.WarpOss" ]; then
    OSS_DB_DIR="$HOME/Library/Application Support/dev.warp.WarpOss"
fi
BACKUP_DIR="$HOME/warp-stable-backup-$(date +%Y%m%d-%H%M%S)"

if pgrep -x stable >/dev/null || pgrep -x warp-oss >/dev/null; then
    echo "ERROR: quit Warp Stable and warp-oss first." >&2
    exit 1
fi

if [ ! -f "$STABLE_DB_DIR/warp.sqlite" ]; then
    echo "ERROR: Stable DB not found at $STABLE_DB_DIR/warp.sqlite" >&2
    exit 1
fi

echo "==> Backing up Stable DB to $BACKUP_DIR"
mkdir -p "$BACKUP_DIR"
cp "$STABLE_DB_DIR/warp.sqlite" "$BACKUP_DIR/"
[ -f "$STABLE_DB_DIR/warp.sqlite-wal" ] && cp "$STABLE_DB_DIR/warp.sqlite-wal" "$BACKUP_DIR/"
[ -f "$STABLE_DB_DIR/warp.sqlite-shm" ] && cp "$STABLE_DB_DIR/warp.sqlite-shm" "$BACKUP_DIR/"

echo "==> Checkpointing WAL into main DB file"
sqlite3 "$STABLE_DB_DIR/warp.sqlite" "PRAGMA wal_checkpoint(TRUNCATE);" >/dev/null

echo "==> Copying DB to warp-oss data dir"
mkdir -p "$OSS_DB_DIR"
# Remove any freshly-created oss DB so the copied one is authoritative.
rm -f "$OSS_DB_DIR/warp.sqlite" "$OSS_DB_DIR/warp.sqlite-wal" "$OSS_DB_DIR/warp.sqlite-shm"
cp "$STABLE_DB_DIR/warp.sqlite" "$OSS_DB_DIR/warp.sqlite"

echo "==> Copying settings/themes/tab configs to ~/.warp-oss"
mkdir -p "$HOME/.warp-oss"
[ -f "$HOME/.warp/settings.toml" ] && cp "$HOME/.warp/settings.toml" "$HOME/.warp-oss/"
for d in themes tab_configs workflows launch_configurations; do
    [ -d "$HOME/.warp/$d" ] && cp -R "$HOME/.warp/$d" "$HOME/.warp-oss/"
done

# Fully-local belt and suspenders: disable telemetry/crash-reporting toggles.
SETTINGS="$HOME/.warp-oss/settings.toml"
touch "$SETTINGS"
if ! grep -q '^\[privacy\]' "$SETTINGS"; then
    printf '\n[privacy]\ntelemetry_enabled = false\ncrash_reporting_enabled = false\n' >> "$SETTINGS"
fi

echo "==> Done. History rows in migrated DB:"
sqlite3 "file:$OSS_DB_DIR/warp.sqlite?mode=ro" "SELECT COUNT(*) FROM commands;"
echo "Launch warp-oss; Diesel will auto-apply pending schema migrations on first start."
echo "NOTE: do not point the old Stable app at the migrated DB (no down-migrations)."
