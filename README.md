# Executor

面向 Windows 的 Android 应用批量处理流水线工具：**导入 → 配置 → 构建 → 产出 → 记录** 五区联动，支持模板化参数/代码注入、Gradle 批量构建、真机日志抓取与全链路操作留痕。

基于 **Tauri 2 + Vue 3 + TypeScript + Rust** 构建，界面使用 Tailwind CSS 4 与 lucide 图标，支持深浅主题。

---

## 目录

- [页面与功能](#页面与功能)
- [全自动化流水线](#全自动化流水线)
- [核心代码架构](#核心代码架构)
- [注入内核语义](#注入内核语义)
- [快捷键系统](#快捷键系统)
- [数据目录布局](#数据目录布局)
- [开发环境与命令](#开发环境与命令)

---

## 页面与功能

### 📥 导入区（Ctrl+1）

Android 项目入库的第一站，只负责「记录 + 复制入库」。

- **队列目录 / 项目目录** 双栏布局；队列卡片展开显示子项目
- 添加项目仅记录信息（名称/包名/下载路径），**记录项目 / 记录全部项目**（右键）才把内容复制到 `import/package/<包名>/`
- 项目三态：未导入 / 导入中 / 已导入；支持修改（改下载路径即刻重复制）、重新导入、从队列移除（保留文件）、删除（真删记录+文件夹）
- 搜索（名称/包名）、按创建日期筛选、批量删除队列/项目
- 右键任意项目卡片可**定位项目**（已导入→导入副本目录，未导入→磁盘源目录）

### ⚙️ 配置区（Ctrl+2）

对导入/磁盘项目做模板化配置与参数、代码注入。

- 添加项目两种来源：**从已导入项目**（自动关联导入区包目录）与**从磁盘目录**（输入配置名称+唯一包名）；同样「添加仅记录、记录才复制」
- **选择配置模板**：保存模板选择并把模板参数 JSON 复制为 `config/parameter/<包名>.json`；可「仅保存」或「保存并开始配置」
- **参数卡片**（左键展开项目卡片）：按参数 JSON 动态生成编辑 UI——日期选择、下拉约束（value_choice）、开关、整数/数字、路径+文件选择、文本；code 类条目提供**函数组合**编辑（函数名/形参/返回类型/语句拼装/规则与回调实参）；支持「从模板重置 / 保存」
- **启动（完善配置）**：首次复制模板 `code/` 目录到项目，然后依次执行 **argument 内核**与 **code 内核**注入
- 项目卡片操作：**刷新**（实时从本地文件重读卡片与参数数据）、启动、编辑信息（磁盘项目可改名/包名并同步重命名目录）、删除（真删配置目录+参数 JSON）
- 项目右键菜单：选择配置模板 / 记录项目 / **重载项目（保留参数）** / **从模板重置代码** / 定位项目
- 搜索、按配置日期筛选、批量删除

### 🔨 构建区（Ctrl+3）

Gradle 批量构建与真机日志中心。

- 队列右键**添加项目**：从配置区（已完善配置的项目）或从磁盘目录，只记录地址不复制
- **Gradle 环境**：设置 → 编译 中管理多版本环境（持久化），工具栏下拉选择
- 构建流程：`<gradle环境>/bin/gradle wrapper` → 项目目录内 `gradlew <任务>`；命令预设 `assembleDebug` / `assembleRelease` / `clean assembleRelease`
- **全部构建**：对话框选择命令与方式——**串行编译**（推荐，可随时停止中断后续）或**并行编译**（全部同时，多项目独立转圈/停止）
- **停止构建**：`taskkill /T /F` 结束整个进程树（gradle 派生的 java 子进程一并终止）
- **构建日志区**：每项目/每设备一个选项卡；150ms 批量流式推送（防 UI 冻结）；构建页保留最近 1000 行、设备页 500 行；日志筛选（命中**独立缓存**不被新日志刷掉）；用户上滚自动暂停跟随；选项卡可移除
- **日志缓存**：每次构建/抓取会话全量落盘 `<workspace>/build/logs/`（自动保留最新 100 个文件），一键打开目录
- **设备日志（USB 调试）**：adb 自动探测（ANDROID_HOME / SDK 默认路径 / PATH）→ 按设备分页抓取 logcat；支持**应用包名过滤**（pidof + `logcat --pid`，应用未启动自动等待、进程结束自动重连）

### 📦 产出区（Ctrl+4）

构建成功后自动收集产物。

- 收集规则：导入类型项目取 `<项目>/output/` 全部文件；其它项目递归扫描 `*.apk`
- 项目信息优先取配置区（名称/包名/**模板名标签**），直接磁盘构建回退构建卡片信息
- **同包名覆盖**：重复构建同一项目只保留最新卡片，不堆积重复
- 卡片按模板页风格设计，**伸缩展开**文件列表：每个文件支持**复制**（资源管理器式剪贴板复制，随处 Ctrl+V 粘贴）与**真删除**
- 搜索（项目/包名/模板/文件名）、按记录日期筛选、**模板标签筛选**、批量删除（全部真删文件，二次确认）

### 📜 记录（Ctrl+5）

全应用操作留痕中心。

- 四个区所有**增/删/改**操作自动记录（后端约 30 处挂接，含级联动作），上限 500 条自动截断
- 卡片含：页面标签（导入绿/配置蓝/构建琥珀/产出紫）+ 操作标签（新增/删除/修改）+ 标题 + 详情 + **子记录**（受影响的项目/文件列表，可逐条删除）+ 时间
- 筛选：搜索（标题/详情/子记录）、日期、操作标签、页面类型，四重叠加；批量删除（仅删历史，绝不动文件）

### 🧩 模板（Ctrl+6）

参数与代码模板库，配置区的注入源。

- 创建/编辑模板类型（名称、类型、介绍），模板即 `templates/<名称>/` 目录
- 右键导入 **代码模板**（目录 → `code/`）与 **参数模板**（JSON → `parameter/<名称>.json`），CodeMirror 编辑器直接修改参数 JSON
- 打开模板目录、删除模板（真删目录，级联清空配置项目的模板引用）

### ⚙️ 设置

- **通用**：窗口关闭行为（询问/托盘/退出）；**全部快捷键自定义**（按键捕获、冲突拒绝、清除绑定、恢复默认）
- **外观**：深色/浅色/跟随系统
- **编译**：Gradle 环境多版本管理（添加/移除安装目录）
- **存储**：数据目录（工作空间）迁移

---

## 全自动化流水线

三个页面右键串联，一条链跑完：

```
导入区队列右键「一键转配置区」
  → 建/复用同名配置队列 + 转入全部已导入项目 + 记录复制 + 自动跳转配置区
配置区队列右键「批量模板配置」
  → 选一次模板 → 逐项目：保存模板 → 开始配置 → 完善配置（内核注入）
  → 单项目失败不中断，汇总成功/失败清单
  → 成功项目自动转入同名构建队列 + 跳转构建区
构建区自动弹出「全部构建」对话框
  → 选构建命令 + 串行/并行 → 开始构建 → 产物自动进产出区
```

全链路**幂等**：同名队列复用、项目按包名/地址去重，可随时重跑补齐。

---

## 核心代码架构

```
src-tauri/src/
├── lib.rs                 # 命令注册、窗口/托盘初始化、状态托管
├── main.rs
├── core/
│   ├── settings.rs        # AppSettings（关闭行为/工作空间/主题/Gradle环境/快捷键）
│   ├── window.rs          # 窗口生命周期、托盘、关闭策略
│   ├── tray.rs            # 系统托盘菜单
│   ├── webview.rs         # WebView2 内存级别、禁用浏览器加速键
│   └── android/
│       ├── argument/      # argument 内核（XML/Gradle/Java 参数注入器）
│       └── code/          # code 内核（场景函数生成与标记区注入）
├── common/
│   ├── storage.rs         # ★ copy_dir_complete：全量复制+重试+逐文件校验
│   └── text.rs
├── imports.rs             # 导入队列
├── android_projects.rs    # 导入区项目（记录/导入/删除/级联同步）
├── configs.rs             # 配置区（队列/项目/模板绑定/参数/记录/级联 helper）
├── builds.rs              # 构建区（队列/构建流程/日志流/LogCache/停止）
├── devices.rs             # adb 设备检测 + logcat 抓取（整机/按应用 pid）
├── outputs.rs             # 产出区（收集/覆盖/真删除/剪贴板复制/级联）
├── records.rs             # 操作记录（log_operation 全局挂接点）
├── templates.rs           # 模板库
└── workspace.rs           # 工作空间分区

src/
├── App.vue                # KeepAlive 页面容器、全局快捷键分发器、导航事件
├── lib/                   # 各模块 invoke 封装 + settings/shortcuts/nav/pipeline/toast/theme
└── components/
    ├── import/ config/ build/ output/ records/ templates/ settings/
    └── AppSelect / AppTitleBar / SettingsModal / ToastContainer ...
```

### 关键机制

| 机制 | 说明 |
|---|---|
| **命令线程规范** | 所有 spawn 进程 / 大文件 IO 的 Tauri 命令一律 `async + spawn_blocking`（同步命令跑主线程会冻结整个 UI） |
| **日志流** | 子进程双管道 → 读取线程 → 共享缓冲 → flusher 每 150ms 合并为多行 chunk 经事件推送，杜绝 IPC 洪泛 |
| **复制完整性** | `copy_dir_complete`：绝不跳过任何文件；单文件占用 300ms 重试；复制后重走源树校验存在性+字节大小；失败清空半成品目录 |
| **跨页级联同步** | 上游删除/改名自动维护下游引用：导入区删项目→配置区清理未记录卡片；模板删除/改名→配置引用清空/跟随；配置区记录/改名/删除→构建卡片与产出记录重指/移除；所有视图 `onActivated` 自动刷新 |
| **构建进程管理** | `BuildRegistry`/`DeviceRegistry` 记录运行中子进程 pid，停止用 `taskkill /T /F` 杀整棵进程树 |
| **剪贴板文件复制** | WinForms `SetFileDropList` 写入 CF_HDROP（PS 5.1 `Set-Clipboard` 不支持文件），写后回读校验 |
| **快捷键** | 集中注册表 + LIFO 分发（后注册的弹层优先），handler 返回 `false` 向下穿透；`AreBrowserAcceleratorKeysEnabled=false` 关闭 WebView2 浏览器加速键 |
| **性能** | KeepAlive 页面缓存；设置弹窗去背景模糊（GPU 禁用时软件模糊极慢）；WebView2 低内存级别 |

---

## 注入内核语义

模板参数 JSON 按 `write_mode` 分为两类条目：

**argument 内核**（参数写入）——值解析链：
`value_override`（裸值继承+循环检测）→ 类型转换 → `value_format`（Java 日期模板 → strftime）→ `value_choice`（可选值校验）→ `value_prefix`（幂等前缀）；
写入器：**xml**（属性/节点）、**gradle**（`key value` DSL 行）、**java**（字段/常量），支持文件复制条目。

**code 内核**（代码注入）——`scenes` 生成 Java 方法：
`public <returnType> <name>(<params>)`；语句两种：`direct` → `callback(args);`，`ruled` → `<class>.<method>(ruleArgs, this::callback);`（class/method 取自 ruleTemplates）；
注入到 `area_name` 成对标记之间，同名方法自动去重；实参 `{"var": name}` 渲染为裸变量；注入前自动生成 `.bak_时间戳` 备份。

---

## 快捷键系统

全部快捷键可在 **设置 → 通用** 自定义（按键捕获、冲突拒绝、解绑、恢复默认），默认：

| 快捷键 | 动作 |
|---|---|
| `Ctrl+S` | 保存 / 创建 / 确定（当前弹层或卡片的主动作） |
| `Esc` | 关闭最上层弹层 |
| `Ctrl+,` | 打开设置 |
| `Ctrl+F` | 聚焦当前页搜索框 |
| `Ctrl+N` | 当前页主创建按钮 |
| `Ctrl+1~6` | 切换 导入/配置/构建/产出/记录/模板 |

---

## 数据目录布局

所有业务数据集中在用户选择的**工作空间**目录：

```
<workspace>/
├── import/                # 导入区
│   ├── queues.json        # 队列
│   ├── android.json       # 项目记录
│   └── package/<包名>/    # 已导入内容
├── config/                # 配置区
│   ├── queues.json
│   ├── package/<包名>/          # 导入项目副本
│   ├── non-local-package/<包名>/ # 磁盘项目副本
│   └── parameter/<包名>.json     # 参数卡片数据
├── build/
│   ├── queues.json
│   └── logs/              # 构建/设备日志全量缓存（保留最新 100 个）
├── output/outputs.json    # 产出记录（产物文件留在项目目录内）
├── records/records.json   # 操作记录（上限 500 条）
└── templates/<模板名>/    # 模板库
    ├── code/
    └── parameter/<模板名>.json
```

应用设置（关闭行为/主题/Gradle 环境/快捷键等）存于系统数据目录的 `settings.json`，迁移数据目录时通过 `bootstrap.json` 锚点跟踪。

---

## 开发环境与命令

### 依赖

- [Rust](https://rustup.rs/)（stable，≥1.77.2 —— .bat 安全包装依赖 std 修复）
- [Node.js](https://nodejs.org/) ≥ 18 + npm
- Windows：WebView2 Runtime（Win11 内置）
- 可选（按功能）：
  - **Android SDK platform-tools**（adb）—— 设备日志抓取
  - **JDK + Gradle 发行版** —— 构建区（在 设置 → 编译 中登记环境目录）

### 命令

```bash
npm install            # 安装前端依赖

npm run tauri dev      # 开发（前端热更新 + Rust 编译）
npm run tauri build    # 打包发布

# 单独校验
npx vue-tsc --noEmit   # 前端类型检查
cd src-tauri && cargo check   # Rust 编译检查
```

> 修改 `tauri.conf.json` / `capabilities/` 后需重启 dev 进程才生效。

---

## License

见 [LICENSE](./LICENSE)。
