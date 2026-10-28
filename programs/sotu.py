import sys
import re

def calculate_hetu_check_character(hetu_input: str) -> str:
    lookup_table = "0123456789ABCDEFHJKLMNPRSTUVWXY"
    
    # Match DDMMYY or DDMMYYYY, optional separator, and 3-digit individual number
    match = re.match(r"^(\d{2})(\d{2})(\d{2}|\d{4})[-+A-FU-Y]?(\d{3})$", hetu_input.strip(), re.IGNORECASE)
    if not match:
        raise ValueError(
            f"Invalid format: '{hetu_input}'. Expected DDMMYY-ZZZ or DDMMYYYY-ZZZ."
        )
        
    day, month, year, individual_num = match.groups()
    
    # If a 4-digit year (e.g., 1956) is given, keep only the last two digits (56)
    year_short = year[-2:]
    
    combined = f"{day}{month}{year_short}{individual_num}"
    remainder = int(combined) % 31
    
    return lookup_table[remainder]

if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Usage: python sotu.py <DDMMYY-ZZZ>")
        sys.exit(1)
        
    try:
        user_input = sys.argv[1]
        check_char = calculate_hetu_check_character(user_input)
        print(f"Check character: {check_char}")
    except ValueError as e:
        print(f"Error: {e}")
        sys.exit(1)

