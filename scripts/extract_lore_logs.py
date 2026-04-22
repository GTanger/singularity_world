import os
import glob
import re

source_dir = "/home/tanger/Desktop/Claude logs"
output_dir = "/home/tanger/Desktop/Claude_Lore_Only"

os.makedirs(output_dir, exist_ok=True)

md_files = glob.glob(os.path.join(source_dir, "*.md"))

total_original_size = 0
total_cleaned_size = 0

for file_path in md_files:
    total_original_size += os.path.getsize(file_path)
    
    with open(file_path, "r", encoding="utf-8") as f:
        lines = f.readlines()
        
    cleaned_lines = []
    in_code_block = False
    
    for line in lines:
        if line.strip().startswith("```"):
            in_code_block = not in_code_block
            continue
            
        if not in_code_block:
            cleaned_lines.append(line)
            
    # Remove excessive blank lines
    cleaned_text = "".join(cleaned_lines)
    cleaned_text = re.sub(r'\n\s*\n', '\n\n', cleaned_text)
    
    filename = os.path.basename(file_path)
    out_path = os.path.join(output_dir, filename)
    
    with open(out_path, "w", encoding="utf-8") as f:
        f.write(cleaned_text)
        
    total_cleaned_size += os.path.getsize(out_path)

print(f"Total files processed: {len(md_files)}")
print(f"Original Size: {total_original_size / 1024 / 1024:.2f} MB")
print(f"Cleaned Size (No Code): {total_cleaned_size / 1024 / 1024:.2f} MB")
print(f"Reduction: {(1 - total_cleaned_size/total_original_size)*100:.2f}%")
