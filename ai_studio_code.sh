find src -name "*.rs" | sort | while read -r f; do
    echo "--- START OF FILE $f ---"
    cat "$f"
    echo -e "\n--- END OF FILE $f ---\n"
done