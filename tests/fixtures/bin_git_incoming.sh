#!/usr/bin/env sh
#
# Mock `git` for `diff --incoming`, happy path.

THIS_SCRIPTS_PARENT_DIR=$(dirname "$0")

# Append: `--incoming` calls Git more than once.
echo "$@" >> $THIS_SCRIPTS_PARENT_DIR/output_git_args.txt

case "$1" in
fetch)
    exit 0
    ;;
diff)
    cat <<'EOF'
diff --git a/.gitconfig b/.gitconfig
index 1111111..2222222 100644
--- a/.gitconfig
+++ b/.gitconfig
@@ -1,2 +1,2 @@
 [user]
-	name = Old Name
+	name = New Name
EOF
    ;;
*)
    exit 1
    ;;
esac
