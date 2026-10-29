# ==============================================================================
# Common & Status
# ==============================================================================
common-on = 开启
common-off = 关闭
common-connect = 连接
common-disconnect = 断开
common-connected = 已连接
common-disconnected = 未连接
common-connecting = 正在连接…
common-disconnecting = 正在断开…
common-cancel = 取消
common-delete = 删除
common-switch = 切换
common-switching = 正在切换…
common-rename = 重命名
common-options = { $param0 }选项
common-status = 状态：{ $param0 }
common-not-reported = 未上报
common-unconfirmed = 未确认
common-current-state = 当前状态：{ $param0 }
common-current-state-unread = 当前状态：尚未读取
common-current-state-syncing = 当前状态：正在同步

# ==============================================================================
# Navigation
# ==============================================================================
nav-home = 主页
nav-sound = 音效
nav-controls = 控制
nav-devices = 设备
nav-main-menu = 主菜单

# ==============================================================================
# Preferences & About & Empty State
# ==============================================================================
pref-preferences = 首选项
pref-general = 常规
pref-language = 语言
pref-language-desc = 选择界面显示语言
pref-language-auto = 跟随系统
pref-language-restart-hint = 重启应用以生效语言更改
pref-app-icon = 应用图标
pref-app-icon-desc = 选择桌面启动器与应用内图标样式
pref-app-icon-default = 头戴式（圆角矩形）
pref-app-icon-circle = 头戴式（圆形）
pref-app-icon-tws = 入耳式（圆角矩形）
pref-app-icon-tws-circle = 入耳式（圆形）
pref-app-icon-applied-hint = 应用图标已更新，桌面任务栏可能需要片刻刷新
pref-app-icon-apply-failed = 更新用户图标失败：{ $param0 }
about-title = 关于 WaveTune
about-description = 面向 Linux 桌面的非官方无线耳机控制中心（支持 OPPO、OnePlus、realme）
empty-state-no-headset-title = 未连接耳机
empty-state-no-headset-description = 请先在系统蓝牙设置中连接耳机。检测到后将自动连接，也可以通过右上角的耳机菜单手动选择。
empty-state-connecting-title = 正在连接耳机

# ==============================================================================
# Channel & Bluetooth Communication
# ==============================================================================
channel-bluetooth-connection = 蓝牙连接
channel-desc = 使用 Linux RFCOMM 私有控制通道与 BlueZ 协议栈通信。支持使用 --probe 命令行参数执行单次只读探测。
channel-and-diagnostics = 通道与诊断
channel-connecting-to = 正在连接 { $param0 } 控制通道…
channel-connected-syncing = 控制通道已建立，正在同步耳机状态
channel-disconnected = 控制通道已断开
channel-error = 控制通道错误：{ $param0 }
channel-established = 控制通道已建立
channel-init-complete = 控制通道初始化完成
channel-not-connected = 控制通道尚未连接
channel-session-ended = 控制通道会话已结束
channel-status-connected = 状态：控制通道已连接
channel-status-disconnected = 状态：控制通道已断开
channel-headset-closed = 耳机已主动关闭 RFCOMM 控制连接
channel-unable-to-open-rfcomm = 无法打开耳机 RFCOMM 控制通道：{ $param0 }
channel-elnk-rfcomm-not-found = 未找到 ELNK SPP 控制服务 RFCOMM 通道
channel-sdp-conflicting-rfcomm = SDP 返回了多个冲突的 ELNK RFCOMM 通道
channel-sdp-attr-search-failed = 无法创建 SDP 属性搜索
channel-sdp-service-search-failed = 无法创建 SDP 服务搜索
channel-sdp-spp-uuid-failed = 无法创建 SPP 服务 UUID
channel-start-task-failed = 无法启动蓝牙后台任务：{ $param0 }

# ==============================================================================
# Device Management
# ==============================================================================
device-title = 设备
device-management = 设备管理
device-information = 设备信息
device-details = 设备详情
device-type = 设备类型
device-type-phone = 手机
device-type-computer = 电脑
device-type-tablet = 平板
device-type-other = 其他设备
device-mac-address = MAC 地址
device-name = 设备名称：{ $param0 }
device-name-placeholder = 设备名称：—
device-product-id = 产品 ID：{ $param0 }
device-model-placeholder = 产品型号：—
device-refresh-list = 刷新设备列表
device-refreshing-list = 正在刷新设备列表…
device-start-discovery-failed = 无法启动设备发现：{ $param0 }
device-stop-connection-failed = 无法停止当前设备连接：{ $param0 }
device-read-failed = 读取设备失败：{ $param0 }
device-status-disconnected = 状态：设备已断开
device-status-discovery-failed = 状态：设备发现失败：{ $param0 }
device-status-no-connected = 状态：未发现已连接设备
device-status-disconnected-generic = 状态：未连接
device-no-supported-connected = 未连接支持的耳机
device-none-connected = 未发现已连接的 OnePlus/OPPO/realme 耳机
device-found-multiple = 发现多个 OnePlus/OPPO/realme 耳机；请在应用设备列表中选择设备
device-please-select = 请先选择一个已发现的设备
device-switching = 正在切换控制设备…
device-unknown-model = 未知型号
device-unknown-model-handshake-only = 未知型号仅验证控制通道握手和产品识别
device-unrecognized-model-read-only = 未识别到已知型号，保持只读模式
device-unnamed = 未命名设备
device-numbered = { $param0 } · 设备 { $param1 }
device-catalog-desc = 型号目录描述兼容配置，不代表硬件已验证；仅开放当前实现且通过设备能力检查的功能。未知型号保持只读。
device-compatibility = 兼容性
device-handshake-missing-caps = 耳机握手响应缺少功能标识
device-product-id-incomplete = 耳机产品 ID 响应不完整
device-event-list-mismatch = 耳机事件列表长度不符
device-event-list-incomplete = 耳机事件列表响应不完整
device-no-declared-events = 未声明事件
device-event-count-mismatch = 事件数量与响应长度不匹配
device-response-missing-status = 响应缺少成功状态或事件数量
device-reading-configuration = 正在读取耳机配置…

# ==============================================================================
# Battery
# ==============================================================================
battery-case = 充电盒
battery-left-ear = 左耳
battery-left-earbud = 左耳
battery-right-ear = 右耳
battery-right-earbud = 右耳
battery-charging = 充电中
battery-last-read = 上次读取的电量
battery-query-no-data = 电量查询未返回任何电量数据
battery-read = 电量读取
battery-wear-notifications = 电量/佩戴状态变更通知

# ==============================================================================
# Wear & Case Lid
# ==============================================================================
wear-detection = 佩戴检测
wear-status = 佩戴状态：{ $param0 }
wear-status-placeholder = 佩戴状态：—
wear-status-not-supported = 佩戴状态：该型号不支持佩戴检测
wear-status-waiting = 佩戴状态：等待设备数据
wear-status-no-data = 佩戴状态查询未返回任何状态数据
wear-in-ear = 已入耳
wear-out-of-ear = 未入耳
wear-in-case = 盒内
wear-in-case-status = { $param0 }在盒内
wear-removed = 已取出
wear-removed-in-ear = { $param0 }已取出、已入耳
wear-removed-out-of-ear = { $param0 }已取出、未入耳
wear-status-unknown = { $param0 }状态未知
wear-case-lid-closed = 盒盖状态：关闭
wear-case-lid-open = 盒盖状态：打开
wear-case-lid-unknown = 盒盖状态：未知
wear-case-closed = 盒盖关闭
wear-case-open = 盒盖打开
wear-case-status-unknown = 盒盖状态未知
wear-case-lid-read = 佩戴/盒盖状态读取
wear-earbuds-not-in-ear = 未检测到耳机入耳；请先佩戴耳机再切换降噪模式
wear-auto-pause-resume = 自动暂停与恢复
wear-auto-pause-resume-desc = 摘下任意耳机时自动暂停媒体播放，重新佩戴后自动恢复播放。
wear-fit-test = 耳塞贴合度检测

# ==============================================================================
# Firmware
# ==============================================================================
firmware-version-placeholder = 固件版本：—
firmware-version-single = 固件版本：{ $param0 }
firmware-version-left-right = 固件版本：左耳 { $param0 } · 右耳 { $param1 }
firmware-version-full = 固件版本：左耳 { $param0 } · 右耳 { $param1 } · 充电盒 { $param2 }
firmware-version-read = 固件版本读取

# ==============================================================================
# Noise Mode
# ==============================================================================
noise-mode-title = 降噪模式
noise-mode-desc = 降噪模式：{ $param0 }
noise-mode-level = 降噪强度
noise-mode-options = 模式选项
noise-mode-adaptive-preference = 自适应偏好
noise-mode-off = 关闭
noise-mode-noise-cancellation = 降噪
noise-mode-transparency = 通透
noise-mode-deep = 深度降噪
noise-mode-moderate = 中度降噪
noise-mode-mild = 轻度降噪
noise-mode-smart = 智能切换
noise-mode-adaptive = 自适应
noise-mode-standard = 标准
noise-mode-less-noise = 更少噪音
noise-mode-more-noise = 更多噪音
noise-mode-vocal-enhancement = 人声增强
noise-mode-not-returned = 降噪模式：设备未返回可识别的当前模式
noise-mode-blocked = 该型号未开放降噪控制，已阻止发送
noise-mode-confirmed-and-readback = 降噪模式已确认并回读
noise-mode-readback-unrecognized = 降噪设置已确认，但耳机回读格式无法识别
noise-mode-confirmed = 耳机已确认降噪模式：{ $param0 }
noise-mode-reported = 耳机已上报降噪模式：{ $param0 }
noise-mode-waiting-confirm = 正在等待耳机确认降噪模式…
noise-mode-send-failed = 无法发送降噪设置命令：{ $param0 }
noise-mode-switch-failed = 切换降噪模式失败：{ $param0 }
noise-mode-reading-and-wear = 正在读取降噪模式和佩戴状态…
noise-mode-read-support-after-connecting = 连接耳机后读取其降噪模式支持情况
noise-mode-profile-declares-available = 型号配置直接声明了可用降噪模式；运行时查询仅作补充，不阻断功能
noise-mode-profile-missing = 型号配置未声明降噪模式
noise-mode-transparency-title = 通透模式
noise-mode-protocol-mode = 协议模式 { $param0 }
noise-mode-numbered = 模式 { $param0 }

# ==============================================================================
# Dual Device Connection
# ==============================================================================
dual-device-title = 双设备连接
dual-device-desc = 同时连接 2 台蓝牙设备，音频随使用场景自动切换。支持在主页管理已配对设备。
dual-device-seamless = 多设备无缝流转
dual-device-turn-on = 开启
dual-device-not-supported = 当前设备不支持双设备管理
dual-device-turned-off = 双设备管理功能已关闭
dual-device-disabled-blocked = 双设备功能未启用，已阻止操作
dual-device-not-enabled = 耳机未启用双设备连接，无法管理已配对设备
dual-device-please-enable = 请先在耳机中启用双设备连接
dual-device-connect = 连接设备
dual-device-disconnect = 断开连接
dual-device-forget = 删除设备
dual-device-this-host = 本机
dual-device-this-host-label = { $param0 }（本机）
dual-device-this-host-cannot-disconnect = 本机用于维持控制连接，不能从此处断开
dual-device-cannot-delete-connected = 不能删除已连接的设备，请先断开连接后再删除
dual-device-already-in-state = 设备已处于目标连接状态
dual-device-not-in-list = 设备已不在耳机上报列表中，请刷新后重试
dual-device-status-confirmed = 设备连接状态已确认
dual-device-removed = 已从耳机移除设备
dual-device-remove-tooltip = 从耳机已配对记录中删除此设备
dual-device-waiting-update = 正在等待耳机更新设备列表…
dual-device-no-devices-reported = 耳机没有上报已配对设备
dual-device-confirmed-operation = 耳机已确认设备操作
dual-device-readback-mismatch = 耳机确认了操作，但回读的设备列表状态不匹配
dual-device-target-missing = 耳机确认了操作，但在回读的设备列表中未找到目标设备
dual-device-deletion-unconfirmed = 耳机确认了删除，但回读的设备列表中仍包含该设备
dual-device-send-connect-failed = 无法发送设备连接命令
dual-device-send-delete-failed = 无法发送设备删除命令
dual-device-operate-failed = 操作设备失败：{ $param0 }
dual-device-read-after-connecting = 连接耳机后读取其已配对设备
dual-device-reading-status = 正在读取双设备连接状态
dual-device-reading-list = 正在读取耳机已配对设备…
dual-device-state-unavailable = 双设备连接状态不可用：{ $param0 }
dual-device-confirm-status-failed = 无法确认双设备连接状态，已禁止操作：{ $param0 }
dual-device-profile-missing = 型号配置未声明双设备连接管理
dual-device-management-blocked = 该型号未开放已配对设备管理，已阻止发送

# ==============================================================================
# Equalizer
# ==============================================================================
equalizer-title = 耳机均衡器
equalizer-desc = 耳机端内置多种调音风格预设，亦可在主页卡片中快速切换当前调音。
equalizer-select-preset = 选择耳机端调音预设风格
equalizer-preset-classic = 经典
equalizer-preset-clear-treble = 清亮高音
equalizer-preset-clear-vocal = 清澈人声
equalizer-preset-bass-boost = 澎湃低音
equalizer-preset-pure-vocal = 纯享人声
equalizer-preset-dynamic-bass = 动感低音
equalizer-preset-dynaudio-classic = 丹拿原声
equalizer-preset-dynaudio-hi-res = 丹拿高解析
equalizer-preset-ultimate-sound = 至臻原音
equalizer-preset-hd-clarity = 高清解析
equalizer-preset-dynaudio-tuning = 丹拿特调
equalizer-custom-title = 自定义均衡器
equalizer-add-custom = 添加自定义 EQ
equalizer-custom-modify = 修改
equalizer-custom-delete = 删除
equalizer-custom-name = 均衡器名称
equalizer-custom-edit-title = 修改自定义 EQ
equalizer-custom-add-title = 添加自定义 EQ
equalizer-custom-save = 保存
equalizer-custom-add = 添加
equalizer-custom-delete-confirm = 确定要删除该自定义均衡器预设吗？
equalizer-custom-max-reached = 已达到最大可保存自定义 EQ 数量
equalizer-preset-numbered = 预设 { $param0 }
equalizer-not-returned = 耳机均衡器预设：设备未返回
equalizer-blocked = 该型号未开放均衡器控制，已阻止发送
equalizer-applied = 耳机已应用均衡器预设
equalizer-waiting-confirm = 正在等待耳机确认均衡器预设…
equalizer-send-failed = 无法发送均衡器设置命令：{ $param0 }
equalizer-switch-failed = 切换均衡器预设失败：{ $param0 }


# ==============================================================================
# Spatial Audio
# ==============================================================================
spatial-audio-title = 空间音频
spatial-audio-desc = 模拟多声道环绕声场体验，支持固定空间音频与实时头部追踪效果。
spatial-audio-effects-desc = 空间环绕与头部追踪效果
spatial-audio-mode-fixed = 固定
spatial-audio-mode-head-tracking = 头部追踪
spatial-audio-not-returned = 空间音频：设备未返回
spatial-audio-blocked = 该型号未开放空间音频控制，已阻止发送
spatial-audio-applied = 耳机已应用空间音频模式
spatial-audio-waiting-confirm = 正在等待耳机确认空间音频模式…
spatial-audio-send-failed = 无法发送空间音频设置命令：{ $param0 }
spatial-audio-switch-failed = 切换空间音频失败：{ $param0 }

# ==============================================================================
# Game Mode
# ==============================================================================
game-mode-title = 游戏模式
game-mode-desc = 降低音频传输延迟，提升音画同步体验
game-mode-card-desc = 降低音频传输延迟，提升音画同步与游戏竞技体验。可在主页卡片快速切换。
game-mode-ultra-low-latency = 超低延迟
game-mode-state-enabled = 已开启游戏模式
game-mode-state-disabled = 已关闭游戏模式
game-mode-not-returned = 游戏模式：设备未返回
game-mode-blocked = 该型号未开放游戏模式控制，已阻止发送
game-mode-waiting-confirm = 正在等待耳机确认游戏模式…
game-mode-send-failed = 无法发送游戏模式设置命令：{ $param0 }
game-mode-switch-failed = 切换游戏模式失败：{ $param0 }

# ==============================================================================
# Bass Engine
# ==============================================================================
bass-engine-title = BassWave 动态低音
bass-engine-summary = 在耳机内部增强低频表现
bass-engine-weak = 弱
bass-engine-strong = 强
bass-engine-not-returned = 动态低音：设备未返回
bass-engine-send-failed = 无法发送低音增强设置命令：{ $param0 }

# ==============================================================================
# Hi-Res Mode
# ==============================================================================
hi-res-title = Hi-Res 高解析模式
hi-res-summary = 启用高规格高清音频传输，呈现更丰富的声音细节
hi-res-not-returned = 高解析模式：设备未返回
hi-res-send-failed = 无法发送高解析模式设置命令：{ $param0 }

# ==============================================================================
# Sound Quality & Audio Effects
# ==============================================================================
sound-quality-and-effects = 音质音效
sound-spatial-and-enhancement = 空间音频与音效增强
sound-hearing-and-bass = 听感与低音增强
sound-audio-enhancement = 音频增强
sound-golden-sound-desc = 支持黄金听感个性化调音补偿、BassWave 动态低音增强与高清音频解码传输。

# ==============================================================================
# Touch Gestures & Controls
# ==============================================================================
controls-touch-gestures = 触控手势
controls-tap-double-tap = 单击 / 双击
controls-triple-tap = 三击操作
controls-slide-adjustment = 滑动调节
controls-long-press-switch = 长按模式切换
controls-gestures-desc = 支持自定义左右耳轻触、双击、三击、长按以及滑动耳柄调节音量等快捷操作。
controls-not-enabled = 该型号未开放设置控制
controls-feature-settings = 功能设置
controls-headset-utilities = 耳机实用工具
controls-find-earbuds = 查找耳机
controls-find-and-seal-desc = 支持播放查找蜂鸣声定位耳机，以及通过麦克风检测耳塞气密性获得最佳降噪效果。
controls-ear-left = 左耳
controls-ear-right = 右耳
controls-gesture-single = 单击耳机 / 按捏
controls-gesture-double = 双击耳机 / 两次按捏
controls-gesture-triple = 三击耳机 / 三次按捏
controls-gesture-hold = 长按耳机 / 长捏耳机柄
controls-gesture-slide = 滑动耳机
controls-gesture-long-hold = 超长按耳机 / 超长捏
controls-gesture-call-single = 单击接听
controls-gesture-call-double = 双击挂断 / 拒接
controls-gesture-call-triple = 三击通话操作
controls-gesture-call-hold = 长按通话操作
controls-gesture-call-other = 其他通话手势
controls-call-gestures-title = 通话状态控制
controls-noise-cycle-title = 长按切换的噪声控制模式
controls-noise-cycle-note = 最少勾选两项以在长按时进行循环切换
controls-noise-cycle-anc = 降噪
controls-noise-cycle-transparency = 通透
controls-noise-cycle-off = 关闭
controls-function-none = 无
controls-function-play-pause = 播放 / 暂停
controls-function-voice-assistant = 语音助手
controls-function-previous = 上一首
controls-function-next = 下一首
controls-function-volume-control = 音量调节
controls-function-noise-control = 噪声控制
controls-function-switch-track = 歌曲切换
controls-function-volume-up = 增大音量
controls-function-volume-down = 减小音量
controls-function-switch-devices = 设备切换
controls-function-game-mode = 游戏模式
controls-function-decline-end-call = 拒接来电 / 挂断电话
controls-function-answer-end-call = 接听 / 挂断电话
controls-function-unknown = 未知功能
controls-head-gestures-title = 头部动作接听
controls-head-gestures-desc = 来电时通过点头或摇头接听、拒接电话。
controls-head-nod-answers = 点头接听，摇头拒接
controls-head-shake-answers = 摇头接听，点头拒接
controls-find-play = 播放声音
controls-find-stop = 停止播放
controls-find-playing-note = 正在播放提示音…放入充电盒或点击停止可关闭声音。
controls-find-in-ear-title = 佩戴提示
controls-find-in-ear-message = 耳机处于入耳佩戴状态，播放大音量响铃可能损伤听力。是否确定继续播放？
controls-find-in-case = 耳机在充电盒内，无法播放响铃
controls-fit-test-title = 耳塞贴合度检测
controls-fit-test-desc = 请佩戴好双耳并在安静环境下进行检测，以确保获得最佳降噪与音质。
controls-fit-test-start = 开始检测
controls-fit-test-running = 正在检测贴合度，请保持安静…
controls-fit-test-good = 贴合良好
controls-fit-test-average = 贴合一般
controls-fit-test-poor = 贴合较差，请调整耳塞
controls-fit-test-failed = 检测失败，请重新测试
controls-fit-test-wear-both = 请先佩戴好左右两只耳机
controls-prompt-volume-title = 提示音音量
controls-prompt-volume-desc = 调节耳机切换模式、电量提示等系统音效的播放音量。
controls-sleep-detection-title = 入睡检测
controls-sleep-detection-desc = 监测到入睡后自动暂停音乐播放以保护听力与电量。
controls-preferred-device-title = 首选连接设备
controls-preferred-device-desc = 开启双设备连接后，在连接范围内耳机将优先回连该设备。
controls-preferred-device-auto = 自动选择


# ==============================================================================
# Probe Diagnostics
# ==============================================================================
probe-device = 设备：{ $param0 }
probe-model-profile = 型号兼容配置：{ $param0 }
probe-product-id = 产品 ID：{ $param0 }
probe-color-id = 颜色编号：{ $param0 }
probe-firmware = 固件版本：左耳 { $param0 }，右耳 { $param1 }，充电盒 { $param2 }
probe-noise-mode-index = 降噪模式索引：{ $param0 }
probe-noise-mode-support = 降噪模式支持信息：{ $param0 }
probe-headset-eq = 耳机均衡器预设：{ $param0 }
probe-spatial-audio = 空间音频：{ $param0 }
probe-game-mode = 游戏模式：{ $param0 }
probe-bass-engine = 动态低音：{ $param0 }（档位 { $param1 }，范围 { $param2 }..{ $param3 }）
probe-bass-engine-simple = 动态低音：{ $param0 }
probe-hi-res = 高解析模式：{ $param0 }
probe-not-reported = { $param0 }：设备未上报
probe-detected-read-only = 已检测只读功能：{ $param0 }
probe-no-queries-confirmed = 未确认任何可用的状态查询
probe-note = 探测提示：{ $param0 }
probe-custom-eq-list = 自定义均衡器：{ $param0 }
probe-custom-eq-none = 自定义均衡器：未添加自定义预设
