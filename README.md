# Rust_for_exif
### 调教AI

- C++不会看不懂，最近在学Rust，就先试试看

---

### 合并远程仓库内容
- 先创建密钥
- `git remote add origin https://你的用户名:你的令牌@github.com/BewiZ/Rust_for_exif.git`
- `git pull origin main --allow-unrelated-histories`拉取远程内容并允许不相关的历史记录
- `git add .`添加所有文件到暂存区
- `git commit -m "合并远程仓库内容"`提交合并
- `git branch -M main`创建 main 分支
- `git push -u origin main`推送代码
- 