# Unity3D Game Development Standards - CODING-STANDARDS-UNITY

**Version**: 2.0.0
**Scope**: Unity3D game development (2D/3D/Mobile/PC/Console/VR/AR)
**Tech Stack**: Unity 2021+, C#, URP/HDRP, Addressables, DOTS
**Last Updated**: 2025-12-25

---

## 🚨 Core Iron Rules (Inherited from common.md)

> **Must follow the four core iron rules from common.md + game.md general game standards**

```
Iron Rule 1: SPEC is the Single Source of Truth (SSOT)
       - Game mechanics must comply with SPEC definitions
       - Prefab, ScriptableObject structure based on SPEC

Iron Rule 2: Smart Reuse and Destroy-Rebuild
       - Existing component fully matches → Direct reuse
       - Partial match → Delete and rebuild, no incremental modifications

Iron Rule 3: Prohibit Incremental Development
       - Prohibit adding new features to old MonoBehaviour
       - Prohibit retaining deprecated Inspector fields

Iron Rule 4: Context7 Research First
       - Use Unity official packages and Asset Store mature plugins
       - Prohibit implementing infrastructure like UI framework and networking yourself
```

---

## 🏗️ Unity Project Structure

### Directory Organization
```
Assets/
├── _Project/              # Project-specific resources (underscore ensures sorting first)
│   ├── Scripts/           # C# scripts
│   │   ├── Core/          # Core systems
│   │   ├── Gameplay/      # Game logic
│   │   ├── UI/            # UI logic
│   │   └── Utilities/     # Utility classes
│   ├── Prefabs/           # Prefabs
│   ├── Scenes/            # Scene files
│   ├── ScriptableObjects/ # Data configuration
│   ├── Materials/         # Materials
│   ├── Textures/          # Textures
│   ├── Audio/             # Audio
│   └── Animations/        # Animations
├── Plugins/               # Third-party plugins
├── Resources/             # Dynamically loaded resources (use sparingly)
├── StreamingAssets/       # Raw assets
└── AddressableAssets/     # Addressables resources
```

### Naming Conventions
- ✅ Scripts: PascalCase (`PlayerController.cs`)
- ✅ Prefabs: PascalCase (`Enemy_Goblin.prefab`)
- ✅ Scenes: PascalCase (`Level_01_Forest.unity`)
- ✅ Materials: m_PascalCase (`m_Character_Skin.mat`)
- ✅ Textures: t_PascalCase (`t_Ground_Diffuse.png`)
- ❌ Prohibit spaces and Chinese naming

---

## 📜 C# Coding Standards

### MonoBehaviour Lifecycle
```csharp
// ✅ Correct lifecycle order
public class PlayerController : MonoBehaviour
{
    // 1. Serialized fields (Inspector visible)
    [SerializeField] private float moveSpeed = 5f;
    [SerializeField] private Transform weaponSlot;

    // 2. Private fields
    private Rigidbody _rigidbody;
    private bool _isGrounded;

    // 3. Properties
    public bool IsAlive { get; private set; }

    // 4. Unity lifecycle methods (in call order)
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

    // 5. Public methods
    public void TakeDamage(float damage) { }

    // 6. Private methods
    private void HandleInput() { }
    private void HandleMovement() { }
}
```

### Code Standards
- ✅ Use `[SerializeField]` instead of `public` fields
- ✅ Private fields use `_` prefix
- ✅ Cache component references (GetComponent only in Awake/Start)
- ✅ Use `TryGetComponent` to avoid null references
- ❌ Prohibit using `Find`, `GetComponent` in Update
- ❌ Prohibit using `GameObject.Find` (use dependency injection or events)

### Events and Communication
```csharp
// ✅ Use C# events or UnityEvent
public class GameEvents : MonoBehaviour
{
    public static event Action<int> OnScoreChanged;
    public static event Action OnGameOver;

    public static void TriggerScoreChanged(int score)
    {
        OnScoreChanged?.Invoke(score);
    }
}

// ✅ Use ScriptableObject event channels
[CreateAssetMenu(menuName = "Events/Game Event")]
public class GameEvent : ScriptableObject
{
    private readonly List<GameEventListener> _listeners = new();

    public void Raise() { /* ... */ }
    public void RegisterListener(GameEventListener listener) { /* ... */ }
}
```

---

## 🎨 Rendering and Performance

### Rendering Pipelines
- ✅ Use URP (Universal Render Pipeline) for mobile
- ✅ Use HDRP for high-end PC/console
- ✅ Use URP 2D Renderer for 2D projects
- ❌ Avoid built-in render pipeline (Legacy)

### Draw Call Optimization
- ✅ Use GPU Instancing
- ✅ Enable Static Batching for static objects
- ✅ Use Dynamic Batching for dynamic objects (vertex count < 300)
- ✅ Use SRP Batcher (URP/HDRP)
- ✅ Merge materials and texture atlases
- ❌ Avoid runtime material modification (use MaterialPropertyBlock)

### LOD and Culling
- ✅ Configure LOD Groups for complex models
- ✅ Enable Occlusion Culling
- ✅ Set reasonable Camera Far Clip Plane
- ✅ Use Layer and Culling Mask to optimize rendering

### Lighting Optimization
- ✅ Use Lightmaps for static lighting
- ✅ Limit real-time light sources on mobile (< 4)
- ✅ Use Light Probes and Reflection Probes
- ❌ Avoid excessive real-time shadows

---

## 📦 Resource Management

### Addressables System
```csharp
// ✅ Use Addressables to load resources
public async Task<GameObject> LoadPrefabAsync(string address)
{
    var handle = Addressables.LoadAssetAsync<GameObject>(address);
    await handle.Task;
    return handle.Result;
}

// ✅ Properly release resources
public void UnloadAsset(AsyncOperationHandle handle)
{
    Addressables.Release(handle);
}
```

### Object Pooling
```csharp
// ✅ Use Unity 2021+ ObjectPool
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

### Resources Folder
- ❌ Avoid using Resources folder (affects startup time)
- ✅ Use Addressables instead
- ⚠️ Only for small resources that must be always available

---

## 🎮 Input System

### New Input System (Input System Package)
```csharp
// ✅ Use Input System Package
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
        // Handle jump
    }
}
```

- ✅ Use Input System Package (not Legacy Input)
- ✅ Configure Input Actions Asset
- ✅ Support multi-platform input (keyboard/gamepad/touch)
- ❌ Prohibit hardcoded keys (use Input Actions)

---

## 🖼️ UI System

### UI Toolkit vs UGUI
- ✅ Use UI Toolkit for new projects (runtime UI)
- ✅ Use UGUI for old projects (Canvas + RectTransform)
- ✅ Use UI Toolkit for editor extensions

### UGUI Optimization
- ✅ Separate dynamic and static Canvas
- ✅ Disable Raycast Target for invisible UI
- ✅ Use Canvas Group to control overall transparency
- ✅ Avoid deeply nested Layout Groups
- ❌ Prohibit per-frame update of unchanged UI elements

### MVP/MVC Pattern
```csharp
// ✅ Separate UI from logic
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

## 🌐 Network and Multiplayer

### Network Framework Selection
- ✅ Small projects: Netcode for GameObjects
- ✅ Large projects: Photon Fusion / Mirror
- ✅ Real-time competitive: Custom UDP + server authority

### Netcode for GameObjects
```csharp
// ✅ Network object synchronization
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
        // Play effect on all clients
    }
}
```

---

## ⚡ Performance Optimization

### Memory Management
- ✅ Use `Span<T>` and `stackalloc` to reduce GC
- ✅ Object pooling for frequently created objects
- ✅ Use `StringBuilder` for string concatenation
- ✅ Avoid boxing (use generics)
- ❌ Prohibit memory allocation in Update

### Profiling Tools
- ✅ Unity Profiler (CPU/GPU/Memory)
- ✅ Frame Debugger (rendering analysis)
- ✅ Memory Profiler Package (memory snapshots)
- ✅ Profile Analyzer (multi-frame comparison)

### Mobile Optimization
- ✅ Target frame rate 30/60 fps
- ✅ Texture compression (ASTC/ETC2)
- ✅ Reduce shader variants
- ✅ Use GPU Instancing
- ✅ Disable unnecessary Quality Settings

---

## 🧪 Testing

### Unit Tests
```csharp
// ✅ Use Unity Test Framework
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

// ✅ Play Mode tests
[UnityTest]
public IEnumerator Player_WhenJumping_BecomesAirborne()
{
    var player = Object.Instantiate(playerPrefab);
    player.Jump();
    yield return new WaitForSeconds(0.1f);
    Assert.IsFalse(player.IsGrounded);
}
```

### Test Coverage
- ✅ Game logic unit tests
- ✅ Play Mode integration tests
- ✅ Performance regression tests
- ✅ Multi-platform compatibility tests

---

## 📋 Unity Development Checklist

### Code Quality
- [ ] Use `[SerializeField]` instead of public fields
- [ ] Cache component references in Awake/Start
- [ ] No Find/GetComponent in Update
- [ ] Use event communication instead of direct references

### Performance
- [ ] Use object pooling
- [ ] Draw call optimization (Batching/Instancing)
- [ ] LOD and culling configuration
- [ ] No memory leaks (Profiler verified)

### Resources
- [ ] Use Addressables for resource management
- [ ] Correct texture compression configuration
- [ ] Audio compression and streaming
- [ ] Mobile resource resolution adaptation

### Architecture
- [ ] UI logic separation (MVP/MVC)
- [ ] ScriptableObject data-driven
- [ ] Event system decoupling
- [ ] Test coverage for core logic

---

## 🏛️ Advanced Architecture Patterns

### Dependency Injection (DI)
```csharp
// ✅ Use VContainer or Zenject
public class GameInstaller : LifetimeScope
{
    protected override void Configure(IContainerBuilder builder)
    {
        builder.Register<IGameService, GameService>(Lifetime.Singleton);
        builder.Register<IPlayerRepository, PlayerRepository>(Lifetime.Scoped);
        builder.RegisterEntryPoint<GameInitializer>();
    }
}

// ✅ Constructor injection
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

### Reactive Programming (UniRx/R3)
```csharp
// ✅ Use Observable for async data streams
public class HealthSystem : MonoBehaviour
{
    private readonly ReactiveProperty<float> _health = new(100f);
    public IReadOnlyReactiveProperty<float> Health => _health;

    private void Start()
    {
        // Automatically update UI when health changes
        _health
            .Where(h => h <= 0)
            .First()
            .Subscribe(_ => OnDeath())
            .AddTo(this);

        // Debounce damage display
        _health
            .Pairwise()
            .Where(pair => pair.Previous > pair.Current)
            .ThrottleFirst(TimeSpan.FromSeconds(0.1f))
            .Subscribe(pair => ShowDamageNumber(pair.Previous - pair.Current))
            .AddTo(this);
    }
}
```

### DOTS/ECS High-Performance Architecture
```csharp
// ✅ ECS components (pure data)
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

// ✅ ECS systems (pure logic)
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

### Command Pattern and Undo System
```csharp
// ✅ Undoable command system
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

## 🔧 Essential Skills for Senior Developers

### Editor Extensions
```csharp
// ✅ Custom Inspector
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

// ✅ Custom PropertyDrawer
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

### Advanced Shader Programming (Shader Graph + HLSL)
```hlsl
// ✅ Custom Shader Graph node
void DistanceField_float(float3 Position, float3 Center, float Radius, out float Distance)
{
    Distance = length(Position - Center) - Radius;
}

// ✅ High-performance vertex animation
void WindAnimation_float(float3 WorldPosition, float Time, float Strength, out float3 Offset)
{
    float phase = dot(WorldPosition, float3(0.5, 0.0, 0.5));
    float wave = sin(Time * 2.0 + phase) * Strength;
    Offset = float3(wave, 0.0, wave * 0.5);
}
```

### Memory Optimization Deep Dive
```csharp
// ✅ Struct memory layout optimization
[StructLayout(LayoutKind.Sequential, Pack = 1)]
public struct OptimizedData
{
    public byte Type;      // 1 byte
    public byte Flags;     // 1 byte
    public short Id;       // 2 bytes
    public float Value;    // 4 bytes
    // Total: 8 bytes, aligned
}

// ✅ Use NativeArray to avoid GC
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

// ✅ Use IJobParallelForTransform for parallel updates
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

### Hot Reload and Fast Iteration
```csharp
// ✅ Configuration system supporting runtime hot reload
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
            // Reload config on main thread
            UnityMainThreadDispatcher.Instance.Enqueue(ReloadConfig);
        }
    }
}
```

---

## 🚨 Common Pitfalls for Senior Developers

### Must Avoid Anti-Patterns
```csharp
// ❌ Wrong: Singleton abuse causes testing difficulties
public class GameManager : MonoBehaviour
{
    public static GameManager Instance;  // Anti-pattern
}

// ✅ Correct: Use dependency injection
public class GameManager : MonoBehaviour
{
    [Inject] private readonly IGameService _gameService;
}

// ❌ Wrong: String concatenation in Update
void Update()
{
    debugText.text = "Score: " + score.ToString();  // Allocates every frame
}

// ✅ Correct: Use StringBuilder or cache
private readonly StringBuilder _sb = new();
void UpdateScoreText()
{
    _sb.Clear();
    _sb.Append("Score: ").Append(score);
    debugText.SetText(_sb);
}

// ❌ Wrong: new WaitForSeconds in coroutine
IEnumerator AttackLoop()
{
    while (true)
    {
        yield return new WaitForSeconds(1f);  // Allocates every time
    }
}

// ✅ Correct: Cache YieldInstruction
private readonly WaitForSeconds _attackDelay = new(1f);
IEnumerator AttackLoop()
{
    while (true)
    {
        yield return _attackDelay;
    }
}
```

### Performance Monitoring Metrics
| Metric | Mobile Target | PC Target |
|------|--------------|-----------|
| Frame Rate | 30-60 fps | 60-144 fps |
| Draw Calls | < 100 | < 500 |
| SetPass Calls | < 50 | < 200 |
| Triangles | < 100K | < 2M |
| Memory Usage | < 1GB | < 4GB |
| GC Allocation | < 1KB/frame | < 10KB/frame |

---

**Unity Development Principles Summary**:
Component Design, Addressables Resources, Object Pooling, Event Decoupling, ScriptableObject Data-Driven, Profiler Optimization, New Input System, URP Rendering Pipeline, Test Coverage, Mobile Adaptation, DI Dependency Injection, Reactive Programming, DOTS High-Performance, Editor Extensions, Zero-Allocation Memory
