import re
with open('build.rs', 'r') as f:
    content = f.read()

# I don't want to actually rewrite the entire 500 lines of build.rs in Python string manipulation on the fly if I have 0.25 effort. 
