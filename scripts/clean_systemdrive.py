import subprocess, os, shutil

os.chdir(r'C:\Users\vladi\Desktop\DeepSeek-Mobile')
d = '%SystemDrive%'

subprocess.run(['git', 'rm', '--cached', '-r', d], check=True)

if os.path.isdir(d):
    shutil.rmtree(d)
    print(f'Deleted dir: {d}')

gitignore = '.gitignore'
with open(gitignore, 'a') as f:
    f.write('\n# Windows system cache files\n')
    f.write('%SystemDrive%/\n')
print('Done')
