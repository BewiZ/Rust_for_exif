# Rust_for_exif
### 调教AI

- C++不会看不懂，最近在学Rust，就先试试看


---
#### 重新使用little_exif(0.6.16)
- 增加了对于png格式的exif读取
  - 主要就是生成`.xmp.xml`，再读取，构建ExifTag信息输出


---

#### 合并远程仓库内容
- 先创建密钥
- `git remote add origin https://你的用户名:你的令牌@github.com/BewiZ/Rust_for_exif.git`
- `git pull origin main --allow-unrelated-histories`拉取远程内容并允许不相关的历史记录
- `git add .`添加所有文件到暂存区
- `git commit -m "合并远程仓库内容"`提交合并
- `git branch -M main`创建 main 分支
- `git push -u origin main`推送代码
