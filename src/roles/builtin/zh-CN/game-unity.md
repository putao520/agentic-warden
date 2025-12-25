# Unity3D 游戏开发规范 - CODING-STANDARDS-UNITY

**版本**: 2.0.0
**适用范围**: Unity3D 游戏开发（2D/3D/移动/PC/主机/VR/AR）
**技术栈**: Unity 2021+、C#、URP/HDRP、Addressables、DOTS
**最后更新**: 2025-12-25

---

## 🚨 核心铁律（继承自 common.md）

> **必须遵循 common.md 的四大核心铁律 + game.md 通用游戏规范**

```
铁律1: SPEC 是唯一真源（SSOT）
       - 游戏机制必须符合 SPEC 定义
       - Prefab、ScriptableObject 结构以 SPEC 为准

铁律2: 智能复用与销毁重建
       - 现有组件完全匹配 → 直接复用
       - 部分匹配 → 删除重建，不做渐进式修改

铁律3: 禁止渐进式开发
       - 禁止在旧 MonoBehaviour 上添加新功能
       - 禁止保留废弃的 Inspector 字段

铁律4: Context7 调研先行
       - 使用 Unity 官方包和 Asset Store 成熟插件
       - 禁止自己实现 UI 框架、网络库等基础设施
```

---

## 🏗️ Unity 项目结构

### 目录组织
```
Assets/
├── _Project/              # 项目专用资源（下划线确保排序靠前）
│   ├── Scripts/           # C# 脚本
│   │   ├── Core/          # 核心系统
│   │   ├── Gameplay/      # 游戏逻辑
│   │   ├── UI/            # UI 逻辑
│   │   └── Utilities/     # 工具类
│   ├── Prefabs/           # 预制体
│   ├── Scenes/            # 场景文件
│   ├── ScriptableObjects/ # 数据配置
│   ├── Materials/         # 材质
│   ├── Textures/          # 纹理
│   ├── Audio/             # 音频
│   └── Animations/        # 动画
├── Plugins/               # 第三方插件
├── Resources/             # 动态加载资源（谨慎使用）
├── StreamingAssets/       # 原始资源
└── AddressableAssets/     # Addressables 资源
```

### 命名规范
- ✅ 脚本：PascalCase（`PlayerController.cs`）
- ✅ Prefab：PascalCase（`Enemy_Goblin.prefab`）
- ✅ 场景：PascalCase（`Level_01_Forest.unity`）
- ✅ 材质：m_PascalCase（`m_Character_Skin.mat`）
- ✅ 纹理：t_PascalCase（`t_Ground_Diffuse.png`）
- ❌ 禁止空格和中文命名

---

## 📜 C# 编码规范

### MonoBehaviour 生命周期
```csharp
// ✅ 正确的生命周期顺序
public class PlayerController : MonoBehaviour
{
    // 1. 序列化字段（Inspector 可见）
    [SerializeField] private float moveSpeed = 5f;
    [SerializeField] private Transform weaponSlot;

    // 2. 私有字段
    private Rigidbody _rigidbody;
    private bool _isGrounded;

    // 3. 属性
    public bool IsAlive { get; private set; }

    // 4. Unity 生命周期方法（按调用顺序）
    private void Awake()
    {
        _rigidbody = GetComponent<Rigidbody>();
    }

    private void Start()
    {
        IsAlive = true;
    }

    private void Update()
    {
        HandleInput();
    }

    private void FixedUpdate()
    {
        HandleMovement();
    }

    // 5. 公共方法
    public void TakeDamage(float damage) { }

    // 6. 私有方法
    private void HandleInput() { }
    private void HandleMovement() { }
}
```

### 代码规范
- ✅ 使用 `[SerializeField]` 而非 `public` 字段
- ✅ 私有字段使用 `_` 前缀
- ✅ 缓存组件引用（`GetComponent` 只在 Awake/Start 调用）
- ✅ 使用 `TryGetComponent` 避免空引用
- ❌ 禁止在 Update 中使用 `Find`、`GetComponent`
- ❌ 禁止使用 `GameObject.Find`（使用依赖注入或事件）

### 事件和通信
```csharp
// ✅ 使用 C# 事件或 UnityEvent
public class GameEvents : MonoBehaviour
{
    public static event Action<int> OnScoreChanged;
    public static event Action OnGameOver;

    public static void TriggerScoreChanged(int score)
    {
        OnScoreChanged?.Invoke(score);
    }
}

// ✅ 使用 ScriptableObject 事件通道
[CreateAssetMenu(menuName = "Events/Game Event")]
public class GameEvent : ScriptableObject
{
    private readonly List<GameEventListener> _listeners = new();

    public void Raise() { /* ... */ }
    public void RegisterListener(GameEventListener listener) { /* ... */ }
}
```

---

## 🎨 渲染和性能

### 渲染管线
- ✅ 移动端使用 URP（Universal Render Pipeline）
- ✅ 高端 PC/主机使用 HDRP
- ✅ 2D 项目使用 URP 2D Renderer
- ❌ 避免内置渲染管线（Legacy）

### Draw Call 优化
- ✅ 使用 GPU Instancing
- ✅ 静态物体启用 Static Batching
- ✅ 动态物体使用 Dynamic Batching（顶点数 < 300）
- ✅ 使用 SRP Batcher（URP/HDRP）
- ✅ 合并材质和纹理图集
- ❌ 避免运行时修改 Material（使用 MaterialPropertyBlock）

### LOD 和剔除
- ✅ 复杂模型配置 LOD Group
- ✅ 启用 Occlusion Culling
- ✅ 设置合理的 Camera Far Clip Plane
- ✅ 使用 Layer 和 Culling Mask 优化渲染

### 光照优化
- ✅ 静态光照使用 Lightmap
- ✅ 移动端限制实时光源数量（< 4）
- ✅ 使用 Light Probes 和 Reflection Probes
- ❌ 避免过多实时阴影

---

## 📦 资源管理

### Addressables 系统
```csharp
// ✅ 使用 Addressables 加载资源
public async Task<GameObject> LoadPrefabAsync(string address)
{
    var handle = Addressables.LoadAssetAsync<GameObject>(address);
    await handle.Task;
    return handle.Result;
}

// ✅ 正确释放资源
public void UnloadAsset(AsyncOperationHandle handle)
{
    Addressables.Release(handle);
}
```

### 对象池
```csharp
// ✅ 使用 Unity 2021+ ObjectPool
private ObjectPool<Bullet> _bulletPool;

private void Awake()
{
    _bulletPool = new ObjectPool<Bullet>(
        createFunc: () => Instantiate(bulletPrefab),
        actionOnGet: bullet => bullet.gameObject.SetActive(true),
        actionOnRelease: bullet => bullet.gameObject.SetActive(false),
        actionOnDestroy: bullet => Destroy(bullet.gameObject),
        defaultCapacity: 20,
        maxSize: 100
    );
}
```

### Resources 文件夹
- ❌ 避免使用 Resources 文件夹（影响启动时间）
- ✅ 使用 Addressables 替代
- ⚠️ 仅用于必须随时可用的小型资源

---

## 🎮 输入系统

### 新输入系统（Input System Package）
```csharp
// ✅ 使用 Input System Package
public class PlayerInput : MonoBehaviour
{
    private PlayerInputActions _inputActions;

    private void Awake()
    {
        _inputActions = new PlayerInputActions();
    }

    private void OnEnable()
    {
        _inputActions.Player.Enable();
        _inputActions.Player.Jump.performed += OnJump;
    }

    private void OnDisable()
    {
        _inputActions.Player.Jump.performed -= OnJump;
        _inputActions.Player.Disable();
    }

    private void OnJump(InputAction.CallbackContext context)
    {
        // 处理跳跃
    }
}
```

- ✅ 使用 Input System Package（非 Legacy Input）
- ✅ 配置 Input Actions Asset
- ✅ 支持多平台输入（键盘/手柄/触摸）
- ❌ 禁止硬编码按键（使用 Input Actions）

---

## 🖼️ UI 系统

### UI Toolkit vs UGUI
- ✅ 新项目使用 UI Toolkit（运行时 UI）
- ✅ 旧项目使用 UGUI（Canvas + RectTransform）
- ✅ 编辑器扩展使用 UI Toolkit

### UGUI 优化
- ✅ 分离动态和静态 Canvas
- ✅ 禁用不可见 UI 的 Raycast Target
- ✅ 使用 Canvas Group 控制整体透明度
- ✅ 避免 Layout Group 嵌套过深
- ❌ 禁止每帧更新不变的 UI 元素

### MVP/MVC 模式
```csharp
// ✅ UI 与逻辑分离
public class HealthBarView : MonoBehaviour
{
    [SerializeField] private Slider healthSlider;

    public void UpdateHealth(float normalizedHealth)
    {
        healthSlider.value = normalizedHealth;
    }
}

public class HealthBarPresenter
{
    private readonly HealthBarView _view;
    private readonly PlayerHealth _model;

    public HealthBarPresenter(HealthBarView view, PlayerHealth model)
    {
        _view = view;
        _model = model;
        _model.OnHealthChanged += OnHealthChanged;
    }

    private void OnHealthChanged(float current, float max)
    {
        _view.UpdateHealth(current / max);
    }
}
```

---

## 🌐 网络和多人游戏

### 网络框架选择
- ✅ 小型项目：Netcode for GameObjects
- ✅ 大型项目：Photon Fusion / Mirror
- ✅ 实时竞技：自研 UDP + 服务器权威

### Netcode for GameObjects
```csharp
// ✅ 网络对象同步
public class NetworkedPlayer : NetworkBehaviour
{
    [SerializeField] private NetworkVariable<int> score = new();

    [ServerRpc]
    public void AddScoreServerRpc(int points)
    {
        score.Value += points;
    }

    [ClientRpc]
    public void PlayEffectClientRpc()
    {
        // 所有客户端播放效果
    }
}
```

---

## ⚡ 性能优化

### 内存管理
- ✅ 使用 `Span<T>` 和 `stackalloc` 减少 GC
- ✅ 对象池化高频创建对象
- ✅ 使用 `StringBuilder` 拼接字符串
- ✅ 避免装箱（使用泛型）
- ❌ 禁止在 Update 中分配内存

### Profiling 工具
- ✅ Unity Profiler（CPU/GPU/Memory）
- ✅ Frame Debugger（渲染分析）
- ✅ Memory Profiler Package（内存快照）
- ✅ Profile Analyzer（多帧对比）

### 移动端优化
- ✅ 目标帧率 30/60 fps
- ✅ 纹理压缩（ASTC/ETC2）
- ✅ 减少 Shader 变体
- ✅ 使用 GPU Instancing
- ✅ 禁用不需要的 Quality Settings

---

## 🧪 测试

### 单元测试
```csharp
// ✅ 使用 Unity Test Framework
[TestFixture]
public class DamageCalculatorTests
{
    [Test]
    public void CalculateDamage_WithCritical_ReturnsDoubleDamage()
    {
        var calculator = new DamageCalculator();
        var result = calculator.Calculate(baseDamage: 100, isCritical: true);
        Assert.AreEqual(200, result);
    }
}

// ✅ Play Mode 测试
[UnityTest]
public IEnumerator Player_WhenJumping_BecomesAirborne()
{
    var player = Object.Instantiate(playerPrefab);
    player.Jump();
    yield return new WaitForSeconds(0.1f);
    Assert.IsFalse(player.IsGrounded);
}
```

### 测试覆盖
- ✅ 游戏逻辑单元测试
- ✅ Play Mode 集成测试
- ✅ 性能回归测试
- ✅ 多平台兼容性测试

---

## 📋 Unity 开发检查清单

### 代码质量
- [ ] 使用 `[SerializeField]` 而非 public 字段
- [ ] 组件引用在 Awake/Start 缓存
- [ ] 无 Update 中的 Find/GetComponent
- [ ] 使用事件通信而非直接引用

### 性能
- [ ] 使用对象池
- [ ] Draw Call 优化（Batching/Instancing）
- [ ] LOD 和剔除配置
- [ ] 内存无泄漏（Profiler 验证）

### 资源
- [ ] 使用 Addressables 管理资源
- [ ] 纹理压缩配置正确
- [ ] 音频压缩和流式播放
- [ ] 移动端资源分辨率适配

### 架构
- [ ] UI 与逻辑分离（MVP/MVC）
- [ ] ScriptableObject 数据驱动
- [ ] 事件系统解耦
- [ ] 测试覆盖核心逻辑

---

## 🏛️ 高级架构模式

### 依赖注入（DI）
```csharp
// ✅ 使用 VContainer 或 Zenject
public class GameInstaller : LifetimeScope
{
    protected override void Configure(IContainerBuilder builder)
    {
        builder.Register<IGameService, GameService>(Lifetime.Singleton);
        builder.Register<IPlayerRepository, PlayerRepository>(Lifetime.Scoped);
        builder.RegisterEntryPoint<GameInitializer>();
    }
}

// ✅ 构造函数注入
public class PlayerController : IInitializable
{
    private readonly IInputService _inputService;
    private readonly IWeaponService _weaponService;

    [Inject]
    public PlayerController(IInputService inputService, IWeaponService weaponService)
    {
        _inputService = inputService;
        _weaponService = weaponService;
    }
}
```

### 响应式编程（UniRx/R3）
```csharp
// ✅ 使用 Observable 处理异步数据流
public class HealthSystem : MonoBehaviour
{
    private readonly ReactiveProperty<float> _health = new(100f);
    public IReadOnlyReactiveProperty<float> Health => _health;

    private void Start()
    {
        // 生命值变化时自动更新 UI
        _health
            .Where(h => h <= 0)
            .First()
            .Subscribe(_ => OnDeath())
            .AddTo(this);

        // 防抖处理伤害显示
        _health
            .Pairwise()
            .Where(pair => pair.Previous > pair.Current)
            .ThrottleFirst(TimeSpan.FromSeconds(0.1f))
            .Subscribe(pair => ShowDamageNumber(pair.Previous - pair.Current))
            .AddTo(this);
    }
}
```

### DOTS/ECS 高性能架构
```csharp
// ✅ ECS 组件（纯数据）
public struct HealthComponent : IComponentData
{
    public float Current;
    public float Max;
}

public struct DamageBuffer : IBufferElementData
{
    public float Amount;
    public Entity Source;
}

// ✅ ECS 系统（纯逻辑）
[BurstCompile]
public partial struct DamageProcessingSystem : ISystem
{
    [BurstCompile]
    public void OnUpdate(ref SystemState state)
    {
        var ecb = new EntityCommandBuffer(Allocator.TempJob);

        foreach (var (health, damageBuffer, entity) in
            SystemAPI.Query<RefRW<HealthComponent>, DynamicBuffer<DamageBuffer>>()
                .WithEntityAccess())
        {
            float totalDamage = 0f;
            foreach (var damage in damageBuffer)
            {
                totalDamage += damage.Amount;
            }

            health.ValueRW.Current -= totalDamage;
            damageBuffer.Clear();

            if (health.ValueRO.Current <= 0)
            {
                ecb.AddComponent<DeadTag>(entity);
            }
        }

        ecb.Playback(state.EntityManager);
        ecb.Dispose();
    }
}
```

### 命令模式与撤销系统
```csharp
// ✅ 可撤销的命令系统
public interface ICommand
{
    void Execute();
    void Undo();
}

public class MoveCommand : ICommand
{
    private readonly Transform _target;
    private readonly Vector3 _newPosition;
    private Vector3 _previousPosition;

    public void Execute()
    {
        _previousPosition = _target.position;
        _target.position = _newPosition;
    }

    public void Undo()
    {
        _target.position = _previousPosition;
    }
}

public class CommandHistory
{
    private readonly Stack<ICommand> _undoStack = new();
    private readonly Stack<ICommand> _redoStack = new();

    public void ExecuteCommand(ICommand command)
    {
        command.Execute();
        _undoStack.Push(command);
        _redoStack.Clear();
    }

    public void Undo()
    {
        if (_undoStack.TryPop(out var command))
        {
            command.Undo();
            _redoStack.Push(command);
        }
    }
}
```

---

## 🔧 资深开发者必备技巧

### 编辑器扩展
```csharp
// ✅ 自定义 Inspector
[CustomEditor(typeof(EnemySpawner))]
public class EnemySpawnerEditor : Editor
{
    public override void OnInspectorGUI()
    {
        var spawner = (EnemySpawner)target;

        EditorGUILayout.LabelField("Spawn Statistics", EditorStyles.boldLabel);
        EditorGUILayout.IntField("Total Spawned", spawner.TotalSpawned);

        if (GUILayout.Button("Force Spawn"))
        {
            spawner.ForceSpawn();
        }

        DrawDefaultInspector();
    }

    private void OnSceneGUI()
    {
        var spawner = (EnemySpawner)target;
        Handles.color = Color.red;
        Handles.DrawWireDisc(spawner.transform.position, Vector3.up, spawner.SpawnRadius);
    }
}

// ✅ 自定义 PropertyDrawer
[CustomPropertyDrawer(typeof(RangeFloatAttribute))]
public class RangeFloatDrawer : PropertyDrawer
{
    public override void OnGUI(Rect position, SerializedProperty property, GUIContent label)
    {
        var range = (RangeFloatAttribute)attribute;
        EditorGUI.Slider(position, property, range.Min, range.Max, label);
    }
}
```

### 高级 Shader 编程（Shader Graph + HLSL）
```hlsl
// ✅ 自定义 Shader Graph 节点
void DistanceField_float(float3 Position, float3 Center, float Radius, out float Distance)
{
    Distance = length(Position - Center) - Radius;
}

// ✅ 高性能顶点动画
void WindAnimation_float(float3 WorldPosition, float Time, float Strength, out float3 Offset)
{
    float phase = dot(WorldPosition, float3(0.5, 0.0, 0.5));
    float wave = sin(Time * 2.0 + phase) * Strength;
    Offset = float3(wave, 0.0, wave * 0.5);
}
```

### 内存优化深度技巧
```csharp
// ✅ 结构体内存布局优化
[StructLayout(LayoutKind.Sequential, Pack = 1)]
public struct OptimizedData
{
    public byte Type;      // 1 byte
    public byte Flags;     // 1 byte
    public short Id;       // 2 bytes
    public float Value;    // 4 bytes
    // Total: 8 bytes, aligned
}

// ✅ 使用 NativeArray 避免 GC
public class BulletSystem : IDisposable
{
    private NativeArray<BulletData> _bullets;
    private TransformAccessArray _transforms;

    public void Initialize(int capacity)
    {
        _bullets = new NativeArray<BulletData>(capacity, Allocator.Persistent);
    }

    public void Dispose()
    {
        _bullets.Dispose();
        _transforms.Dispose();
    }
}

// ✅ 使用 IJobParallelForTransform 并行更新
[BurstCompile]
public struct BulletUpdateJob : IJobParallelForTransform
{
    public float DeltaTime;
    [ReadOnly] public NativeArray<BulletData> Bullets;

    public void Execute(int index, TransformAccess transform)
    {
        var bullet = Bullets[index];
        transform.position += bullet.Velocity * DeltaTime;
    }
}
```

### 热重载和快速迭代
```csharp
// ✅ 支持运行时热重载的配置系统
public class HotReloadableConfig : ScriptableObject
{
    [SerializeField] private string configPath;
    private FileSystemWatcher _watcher;

    private void OnEnable()
    {
        #if UNITY_EDITOR
        _watcher = new FileSystemWatcher(Path.GetDirectoryName(configPath));
        _watcher.Changed += OnConfigChanged;
        _watcher.EnableRaisingEvents = true;
        #endif
    }

    private void OnConfigChanged(object sender, FileSystemEventArgs e)
    {
        if (e.Name == Path.GetFileName(configPath))
        {
            // 在主线程重新加载配置
            UnityMainThreadDispatcher.Instance.Enqueue(ReloadConfig);
        }
    }
}
```

---

## 🚨 资深开发者常见陷阱

### 必须避免的反模式
```csharp
// ❌ 错误：Singleton 滥用导致测试困难
public class GameManager : MonoBehaviour
{
    public static GameManager Instance;  // 反模式
}

// ✅ 正确：使用依赖注入
public class GameManager : MonoBehaviour
{
    [Inject] private readonly IGameService _gameService;
}

// ❌ 错误：Update 中字符串拼接
void Update()
{
    debugText.text = "Score: " + score.ToString();  // 每帧分配
}

// ✅ 正确：使用 StringBuilder 或缓存
private readonly StringBuilder _sb = new();
void UpdateScoreText()
{
    _sb.Clear();
    _sb.Append("Score: ").Append(score);
    debugText.SetText(_sb);
}

// ❌ 错误：协程中 new WaitForSeconds
IEnumerator AttackLoop()
{
    while (true)
    {
        yield return new WaitForSeconds(1f);  // 每次分配
    }
}

// ✅ 正确：缓存 YieldInstruction
private readonly WaitForSeconds _attackDelay = new(1f);
IEnumerator AttackLoop()
{
    while (true)
    {
        yield return _attackDelay;
    }
}
```

### 性能监控指标
| 指标 | 移动端目标 | PC端目标 |
|------|-----------|---------|
| 帧率 | 30-60 fps | 60-144 fps |
| Draw Calls | < 100 | < 500 |
| SetPass Calls | < 50 | < 200 |
| 三角面数 | < 100K | < 2M |
| 内存使用 | < 1GB | < 4GB |
| GC 分配 | < 1KB/帧 | < 10KB/帧 |

---

**Unity 开发原则总结**：
组件化设计、Addressables资源、对象池化、事件解耦、ScriptableObject数据驱动、Profiler优化、新输入系统、URP渲染管线、测试覆盖、移动端适配、DI依赖注入、响应式编程、DOTS高性能、编辑器扩展、内存零分配
