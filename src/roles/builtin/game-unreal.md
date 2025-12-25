# Unreal Engine 游戏开发规范 - CODING-STANDARDS-UNREAL

**版本**: 2.0.0
**适用范围**: Unreal Engine 游戏开发（2D/3D/移动/PC/主机/VR）
**技术栈**: UE5+、C++、Blueprints、Niagara、Lumen/Nanite
**最后更新**: 2025-12-25

---

## 🚨 核心铁律（继承自 common.md）

> **必须遵循 common.md 的四大核心铁律 + game.md 通用游戏规范**

```
铁律1: SPEC 是唯一真源（SSOT）
       - 游戏机制必须符合 SPEC 定义
       - Actor、Component、DataAsset 结构以 SPEC 为准

铁律2: 智能复用与销毁重建
       - 现有类完全匹配 → 直接复用
       - 部分匹配 → 删除重建，不做渐进式修改

铁律3: 禁止渐进式开发
       - 禁止在旧 Actor 上添加新功能
       - 禁止保留废弃的 UPROPERTY 字段

铁律4: Context7 调研先行
       - 使用 UE 官方插件和 Marketplace 成熟资产
       - 禁止自己实现 GAS、网络同步等核心系统
```

---

## 🏗️ 项目结构

### 目录组织
```
Content/
├── _Game/                  # 项目专用资源
│   ├── Blueprints/         # 蓝图类
│   │   ├── Core/           # 核心系统
│   │   ├── Characters/     # 角色
│   │   ├── AI/             # AI 行为
│   │   └── UI/             # UMG 控件
│   ├── Maps/               # 关卡地图
│   ├── DataAssets/         # 数据资产
│   ├── Materials/          # 材质
│   ├── Textures/           # 纹理
│   ├── Meshes/             # 模型
│   ├── Animations/         # 动画
│   ├── Audio/              # 音频
│   ├── Effects/            # 特效（Niagara）
│   └── UI/                 # UI 资源
├── Plugins/                # 项目插件
└── Developers/             # 开发者临时资源（不提交）

Source/
├── MyGame/
│   ├── Public/             # 头文件
│   │   ├── Core/
│   │   ├── Characters/
│   │   ├── Weapons/
│   │   └── UI/
│   ├── Private/            # 实现文件
│   └── MyGame.Build.cs     # 模块配置
└── MyGameEditor/           # 编辑器模块
```

### 命名规范
- ✅ C++ 类：前缀标识类型
  - `A` - Actor（`AMyCharacter`）
  - `U` - UObject（`UHealthComponent`）
  - `F` - 结构体/值类型（`FDamageInfo`）
  - `E` - 枚举（`EWeaponType`）
  - `I` - 接口（`IDamageable`）
  - `T` - 模板（`TArray`）
- ✅ 蓝图：BP_PascalCase（`BP_Player`）
- ✅ 材质：M_PascalCase（`M_Character_Skin`）
- ✅ 纹理：T_PascalCase（`T_Ground_Diffuse`）
- ❌ 禁止空格和中文命名

---

## 📜 C++ 编码规范

### 类声明结构
```cpp
// ✅ 正确的类结构
UCLASS(BlueprintType, Blueprintable)
class MYGAME_API AMyCharacter : public ACharacter
{
    GENERATED_BODY()

public:
    // 1. 构造函数
    AMyCharacter();

    // 2. 公共方法
    UFUNCTION(BlueprintCallable, Category = "Combat")
    void TakeDamage(float Damage, AActor* DamageCauser);

protected:
    // 3. 生命周期方法
    virtual void BeginPlay() override;
    virtual void Tick(float DeltaTime) override;

    // 4. 受保护属性（蓝图可访问）
    UPROPERTY(EditDefaultsOnly, BlueprintReadOnly, Category = "Stats")
    float MaxHealth = 100.f;

    UPROPERTY(BlueprintReadOnly, Category = "Stats")
    float CurrentHealth;

private:
    // 5. 私有组件
    UPROPERTY(VisibleAnywhere)
    TObjectPtr<UHealthComponent> HealthComponent;

    // 6. 私有方法
    void InitializeComponents();
};
```

### UPROPERTY 说明符
```cpp
// ✅ 常用 UPROPERTY 配置
UPROPERTY(EditAnywhere)           // 编辑器可编辑
UPROPERTY(EditDefaultsOnly)       // 仅默认值可编辑
UPROPERTY(VisibleAnywhere)        // 编辑器可见不可编辑
UPROPERTY(BlueprintReadOnly)      // 蓝图只读
UPROPERTY(BlueprintReadWrite)     // 蓝图读写
UPROPERTY(Replicated)             // 网络同步
UPROPERTY(ReplicatedUsing=OnRep_Health)  // 同步回调

// ✅ 组合使用
UPROPERTY(EditDefaultsOnly, BlueprintReadOnly, Category = "Combat")
float BaseDamage = 10.f;
```

### UFUNCTION 说明符
```cpp
// ✅ 常用 UFUNCTION 配置
UFUNCTION(BlueprintCallable)              // 蓝图可调用
UFUNCTION(BlueprintPure)                  // 纯函数（无副作用）
UFUNCTION(BlueprintImplementableEvent)    // 蓝图实现
UFUNCTION(BlueprintNativeEvent)           // C++ 默认实现，蓝图可覆盖
UFUNCTION(Server, Reliable)               // 服务器 RPC
UFUNCTION(Client, Reliable)               // 客户端 RPC
UFUNCTION(NetMulticast, Unreliable)       // 多播 RPC
```

### 智能指针
```cpp
// ✅ 使用 UE 智能指针
TObjectPtr<UObject> ObjectPtr;           // UObject 指针（UE5）
TWeakObjectPtr<AActor> WeakActor;        // 弱引用
TSharedPtr<FMyStruct> SharedStruct;      // 共享指针（非 UObject）
TUniquePtr<FMyStruct> UniqueStruct;      // 唯一指针

// ✅ 软引用（延迟加载）
UPROPERTY(EditDefaultsOnly)
TSoftObjectPtr<UTexture2D> LazyTexture;

UPROPERTY(EditDefaultsOnly)
TSoftClassPtr<AActor> LazyActorClass;
```

---

## 🎨 蓝图规范

### 蓝图组织
- ✅ 使用 Collapsed Nodes 整理复杂逻辑
- ✅ 使用 Comments 标注功能区域
- ✅ 使用 Reroute Nodes 整理连线
- ✅ 复杂逻辑封装到 Functions/Macros
- ❌ 禁止意大利面条式蓝图

### C++ 与蓝图协作
```cpp
// ✅ C++ 定义核心逻辑，蓝图扩展
UCLASS(Abstract, Blueprintable)
class MYGAME_API AWeaponBase : public AActor
{
    GENERATED_BODY()

public:
    // C++ 实现核心逻辑
    UFUNCTION(BlueprintCallable)
    void Fire();

protected:
    // 蓝图实现特定效果
    UFUNCTION(BlueprintImplementableEvent)
    void OnFire();

    // C++ 默认实现，蓝图可覆盖
    UFUNCTION(BlueprintNativeEvent)
    void PlayFireEffect();
};
```

### 蓝图使用场景
- ✅ 快速原型和迭代
- ✅ 美术/策划可调参数
- ✅ 动画和特效逻辑
- ✅ UI 交互逻辑
- ❌ 复杂算法和性能关键代码

---

## 🌐 网络和多人游戏

### 复制系统
```cpp
// ✅ 属性复制
UPROPERTY(Replicated)
float Health;

// ✅ 复制条件
UPROPERTY(ReplicatedUsing = OnRep_Health)
float Health;

void GetLifetimeReplicatedProps(TArray<FLifetimeProperty>& OutLifetimeProps) const override
{
    Super::GetLifetimeReplicatedProps(OutLifetimeProps);
    DOREPLIFETIME_CONDITION(AMyCharacter, Health, COND_OwnerOnly);
}

UFUNCTION()
void OnRep_Health()
{
    // 客户端收到更新时调用
    UpdateHealthUI();
}
```

### RPC 模式
```cpp
// ✅ 服务器 RPC（客户端调用，服务器执行）
UFUNCTION(Server, Reliable, WithValidation)
void ServerRPC_Fire();
bool ServerRPC_Fire_Validate() { return true; }
void ServerRPC_Fire_Implementation() { /* ... */ }

// ✅ 客户端 RPC（服务器调用，客户端执行）
UFUNCTION(Client, Reliable)
void ClientRPC_ShowDamageNumber(float Damage);

// ✅ 多播 RPC（服务器调用，所有客户端执行）
UFUNCTION(NetMulticast, Unreliable)
void MulticastRPC_PlayExplosion();
```

### 网络权威
- ✅ 服务器权威模式
- ✅ 客户端预测 + 服务器验证
- ✅ 使用 `HasAuthority()` 检查权限
- ❌ 禁止客户端直接修改复制属性

---

## ⚔️ Gameplay Ability System (GAS)

### 核心概念
```cpp
// ✅ Ability System Component
UPROPERTY(VisibleAnywhere, BlueprintReadOnly)
TObjectPtr<UAbilitySystemComponent> AbilitySystemComponent;

// ✅ Gameplay Ability
UCLASS()
class UGA_FireWeapon : public UGameplayAbility
{
    GENERATED_BODY()

    virtual void ActivateAbility(
        const FGameplayAbilitySpecHandle Handle,
        const FGameplayAbilityActorInfo* ActorInfo,
        const FGameplayAbilityActivationInfo ActivationInfo,
        const FGameplayEventData* TriggerEventData) override;

    virtual bool CanActivateAbility(
        const FGameplayAbilitySpecHandle Handle,
        const FGameplayAbilityActorInfo* ActorInfo,
        const FGameplayTagContainer* SourceTags,
        const FGameplayTagContainer* TargetTags,
        FGameplayTagContainer* OptionalRelevantTags) const override;
};

// ✅ Gameplay Effect
UCLASS()
class UGE_DamageBase : public UGameplayEffect
{
    // 配置伤害、持续时间、修改器等
};
```

### GAS 最佳实践
- ✅ 使用 Gameplay Tags 管理状态
- ✅ 使用 Gameplay Effects 处理属性修改
- ✅ 使用 Gameplay Cues 处理特效
- ✅ 使用 Target Data 传递目标信息
- ❌ 禁止绕过 GAS 直接修改属性

---

## 🎨 渲染和性能

### UE5 特性
- ✅ Nanite 虚拟几何体（静态网格）
- ✅ Lumen 全局光照
- ✅ Virtual Shadow Maps
- ✅ World Partition 大世界流送

### 性能优化
```cpp
// ✅ 使用 Stat 命令分析
DECLARE_STATS_GROUP(TEXT("MyGame"), STATGROUP_MyGame, STATCAT_Advanced);
DECLARE_CYCLE_STAT(TEXT("Update Combat"), STAT_UpdateCombat, STATGROUP_MyGame);

void AMyCharacter::UpdateCombat()
{
    SCOPE_CYCLE_COUNTER(STAT_UpdateCombat);
    // ...
}
```

### 内存管理
- ✅ 使用 Asset Manager 管理资源
- ✅ 配置 Primary Asset Types
- ✅ 使用 Soft References 延迟加载
- ✅ 使用 Streaming Levels 流送

### 移动端优化
- ✅ 使用 Mobile Forward Renderer
- ✅ 纹理压缩（ASTC/ETC2）
- ✅ 减少 Draw Calls
- ✅ 禁用高级渲染特性

---

## 🖼️ UI 系统 (UMG)

### Widget 结构
```cpp
// ✅ C++ Widget 基类
UCLASS()
class MYGAME_API UHealthBarWidget : public UUserWidget
{
    GENERATED_BODY()

public:
    UFUNCTION(BlueprintCallable)
    void SetHealth(float NormalizedHealth);

protected:
    virtual void NativeConstruct() override;

    UPROPERTY(meta = (BindWidget))
    TObjectPtr<UProgressBar> HealthBar;

    UPROPERTY(meta = (BindWidgetOptional))
    TObjectPtr<UTextBlock> HealthText;
};
```

### UI 最佳实践
- ✅ C++ 定义逻辑，蓝图定义布局
- ✅ 使用 `meta = (BindWidget)` 绑定控件
- ✅ 使用 Common UI 插件（游戏手柄支持）
- ✅ 使用 Widget Component 3D UI
- ❌ 禁止在 Tick 中更新静态 UI

---

## 🎵 音频系统

### MetaSound（UE5）
- ✅ 使用 MetaSound 创建程序化音频
- ✅ 使用 Sound Classes 管理音量
- ✅ 使用 Sound Attenuation 3D 音效
- ✅ 使用 Audio Modulation 动态调制

### 音频优化
- ✅ 使用 Sound Concurrency 限制并发
- ✅ 流式播放长音乐
- ✅ 使用音频池化
- ❌ 避免同时播放过多音效

---

## 🧪 测试

### Automation Testing
```cpp
// ✅ 自动化测试
IMPLEMENT_SIMPLE_AUTOMATION_TEST(
    FDamageCalculatorTest,
    "MyGame.Combat.DamageCalculator",
    EAutomationTestFlags::ApplicationContextMask | EAutomationTestFlags::ProductFilter
)

bool FDamageCalculatorTest::RunTest(const FString& Parameters)
{
    FDamageCalculator Calculator;
    float Result = Calculator.Calculate(100.f, true);
    TestEqual("Critical damage should double", Result, 200.f);
    return true;
}
```

### 测试类型
- ✅ Unit Tests（C++ 逻辑测试）
- ✅ Functional Tests（游戏内测试）
- ✅ Screenshot Tests（视觉回归测试）
- ✅ Gauntlet（自动化性能测试）

---

## 📋 Unreal 开发检查清单

### 代码质量
- [ ] 遵循 UE 命名规范（A/U/F/E/I 前缀）
- [ ] 正确使用 UPROPERTY/UFUNCTION 说明符
- [ ] 使用 TObjectPtr 和智能指针
- [ ] C++ 核心逻辑 + 蓝图扩展

### 网络
- [ ] 属性复制配置正确
- [ ] RPC 权限验证
- [ ] 服务器权威模式
- [ ] 网络性能优化

### 性能
- [ ] 使用 Stat 命令分析
- [ ] 资源流送配置
- [ ] Draw Call 优化
- [ ] 移动端适配

### 架构
- [ ] GAS 技能系统（复杂战斗）
- [ ] UI 逻辑分离
- [ ] Data Assets 数据驱动
- [ ] 自动化测试覆盖

---

## 🏛️ 高级架构模式

### 模块化游戏框架（Modular Game Features）
```cpp
// ✅ 使用 Game Features 和 Modular Gameplay 插件
UCLASS()
class MYGAME_API UMyGameFeatureAction_AddAbilities : public UGameFeatureAction
{
    GENERATED_BODY()

public:
    virtual void OnGameFeatureActivating(FGameFeatureActivatingContext& Context) override;
    virtual void OnGameFeatureDeactivating(FGameFeatureDeactivatingContext& Context) override;

protected:
    UPROPERTY(EditAnywhere, Category = "Abilities")
    TArray<TSubclassOf<UGameplayAbility>> AbilitiesToAdd;
};

// ✅ 通过 GameplayTags 管理模块状态
namespace MyGameTags
{
    UE_DEFINE_GAMEPLAY_TAG(Feature_Combat, "Feature.Combat");
    UE_DEFINE_GAMEPLAY_TAG(Feature_Stealth, "Feature.Stealth");
    UE_DEFINE_GAMEPLAY_TAG(Feature_Vehicle, "Feature.Vehicle");
}
```

### 高级 GAS 架构
```cpp
// ✅ Attribute Set 组织
UCLASS()
class MYGAME_API UMyAttributeSet : public UAttributeSet
{
    GENERATED_BODY()

public:
    // 使用宏简化属性定义
    ATTRIBUTE_ACCESSORS(UMyAttributeSet, Health);
    ATTRIBUTE_ACCESSORS(UMyAttributeSet, MaxHealth);
    ATTRIBUTE_ACCESSORS(UMyAttributeSet, Damage);

    // 属性变化前拦截
    virtual void PreAttributeChange(const FGameplayAttribute& Attribute, float& NewValue) override;

    // 属性变化后处理
    virtual void PostGameplayEffectExecute(const FGameplayEffectModCallbackData& Data) override;

protected:
    UPROPERTY(BlueprintReadOnly, ReplicatedUsing = OnRep_Health)
    FGameplayAttributeData Health;

    UFUNCTION()
    void OnRep_Health(const FGameplayAttributeData& OldHealth);

private:
    void HandleHealthChanged(const FGameplayEffectModCallbackData& Data);
    void HandleDamage(const FGameplayEffectModCallbackData& Data);
};

// ✅ Gameplay Effect Execution Calculation
UCLASS()
class MYGAME_API UDamageExecCalc : public UGameplayEffectExecutionCalculation
{
    GENERATED_BODY()

public:
    UDamageExecCalc();

    virtual void Execute_Implementation(
        const FGameplayEffectCustomExecutionParameters& ExecutionParams,
        FGameplayEffectCustomExecutionOutput& OutExecutionOutput) const override;

protected:
    FGameplayEffectAttributeCaptureDefinition DamageCapture;
    FGameplayEffectAttributeCaptureDefinition ArmorCapture;
};
```

### 高级网络预测
```cpp
// ✅ 自定义移动预测
UCLASS()
class MYGAME_API UMyCharacterMovementComponent : public UCharacterMovementComponent
{
    GENERATED_BODY()

public:
    // 自定义移动模式
    virtual void PhysCustom(float DeltaTime, int32 Iterations) override;

    // 客户端预测
    virtual void MoveAutonomous(
        float ClientTimeStamp,
        float DeltaTime,
        uint8 CompressedFlags,
        const FVector& NewAccel) override;

    // 服务器校正
    virtual void ClientAdjustPosition(
        float TimeStamp,
        FVector NewLoc,
        FVector NewVel,
        UPrimitiveComponent* NewBase,
        FName NewBaseBoneName,
        bool bHasBase,
        bool bBaseRelativePosition,
        uint8 ServerMovementMode) override;

protected:
    // 保存移动状态用于回滚
    virtual void SaveMoveState();
    virtual void RestoreMoveState();
};

// ✅ Gameplay Prediction Key
void AMyCharacter::PerformAbility()
{
    if (HasAuthority())
    {
        // 服务器直接执行
        ExecuteAbility();
    }
    else
    {
        // 客户端预测
        FScopedPredictionWindow ScopedPrediction(AbilitySystemComponent);
        FPredictionKey PredictionKey = AbilitySystemComponent->GetPredictionKeyForNewAction();

        ExecuteAbility_Predicted(PredictionKey);
        ServerExecuteAbility(PredictionKey);
    }
}
```

### 插件架构设计
```cpp
// ✅ 模块化插件结构
// MyPlugin.uplugin
{
    "Modules": [
        {
            "Name": "MyPluginRuntime",
            "Type": "Runtime",
            "LoadingPhase": "Default"
        },
        {
            "Name": "MyPluginEditor",
            "Type": "Editor",
            "LoadingPhase": "PostEngineInit"
        }
    ],
    "Plugins": [
        {
            "Name": "GameplayAbilities",
            "Enabled": true
        }
    ]
}

// ✅ 模块接口
class MYPLUGIN_API IMyPluginInterface : public IModuleInterface
{
public:
    static IMyPluginInterface& Get()
    {
        return FModuleManager::LoadModuleChecked<IMyPluginInterface>("MyPluginRuntime");
    }

    virtual void RegisterCustomAssetType(UClass* AssetClass) = 0;
    virtual TSharedPtr<IMyService> GetService() const = 0;
};
```

---

## 🔧 资深开发者必备技巧

### 自定义 Slate UI
```cpp
// ✅ 高性能自定义 Slate 控件
class SMyCustomWidget : public SCompoundWidget
{
public:
    SLATE_BEGIN_ARGS(SMyCustomWidget)
        : _Text()
        , _OnClicked()
    {}
        SLATE_ATTRIBUTE(FText, Text)
        SLATE_EVENT(FOnClicked, OnClicked)
    SLATE_END_ARGS()

    void Construct(const FArguments& InArgs);

    virtual int32 OnPaint(
        const FPaintArgs& Args,
        const FGeometry& AllottedGeometry,
        const FSlateRect& MyCullingRect,
        FSlateWindowElementList& OutDrawElements,
        int32 LayerId,
        const FWidgetStyle& InWidgetStyle,
        bool bParentEnabled) const override;

    virtual FReply OnMouseButtonDown(const FGeometry& MyGeometry, const FPointerEvent& MouseEvent) override;

private:
    FSlateBrush CustomBrush;
    TAttribute<FText> Text;
    FOnClicked OnClicked;
};
```

### 高级材质系统
```cpp
// ✅ 运行时材质实例管理
UCLASS()
class MYGAME_API UMaterialManager : public UObject
{
    GENERATED_BODY()

public:
    UMaterialInstanceDynamic* GetOrCreateMID(UMaterialInterface* Parent, FName Identifier);
    void UpdateMaterialParameter(FName Identifier, FName ParameterName, float Value);
    void BatchUpdateMaterials(const TArray<FMaterialParameterUpdate>& Updates);

private:
    UPROPERTY()
    TMap<FName, TObjectPtr<UMaterialInstanceDynamic>> MaterialCache;

    // 使用 Material Parameter Collection 批量更新
    UPROPERTY()
    TObjectPtr<UMaterialParameterCollection> GlobalMPC;
};

// ✅ 程序化材质生成
void GenerateProceduralTexture(UTexture2D* Texture, TFunction<FColor(int32 X, int32 Y)> Generator)
{
    FTexture2DMipMap& Mip = Texture->GetPlatformData()->Mips[0];
    void* Data = Mip.BulkData.Lock(LOCK_READ_WRITE);
    FColor* Colors = static_cast<FColor*>(Data);

    const int32 Width = Texture->GetSizeX();
    const int32 Height = Texture->GetSizeY();

    ParallelFor(Height, [&](int32 Y)
    {
        for (int32 X = 0; X < Width; ++X)
        {
            Colors[Y * Width + X] = Generator(X, Y);
        }
    });

    Mip.BulkData.Unlock();
    Texture->UpdateResource();
}
```

### Niagara 高级特效
```cpp
// ✅ 程序化 Niagara 系统
UCLASS()
class MYGAME_API UNiagaraEffectManager : public UObject
{
    GENERATED_BODY()

public:
    void SpawnEffect(UNiagaraSystem* System, const FTransform& Transform, const FNiagaraEffectParams& Params);
    void UpdateEffectParameter(UNiagaraComponent* Component, FName ParameterName, const FVector& Value);

    // 对象池化 Niagara 组件
    UNiagaraComponent* AcquireComponent(UNiagaraSystem* System);
    void ReleaseComponent(UNiagaraComponent* Component);

private:
    TMap<UNiagaraSystem*, TArray<TObjectPtr<UNiagaraComponent>>> ComponentPools;
};

// ✅ Data Interface 自定义数据源
UCLASS()
class MYGAME_API UNiagaraDI_CustomData : public UNiagaraDataInterface
{
    GENERATED_BODY()

public:
    virtual void GetFunctions(TArray<FNiagaraFunctionSignature>& OutFunctions) override;
    virtual void GetVMExternalFunction(
        const FVMExternalFunctionBindingInfo& BindingInfo,
        void* InstanceData,
        FVMExternalFunction& OutFunc) override;
};
```

### 高级 AI 系统（EQS + Behavior Tree）
```cpp
// ✅ 自定义 EQS Generator
UCLASS()
class MYGAME_API UEnvQueryGenerator_CoverPoints : public UEnvQueryGenerator
{
    GENERATED_BODY()

public:
    UEnvQueryGenerator_CoverPoints();

    virtual void GenerateItems(FEnvQueryInstance& QueryInstance) const override;

protected:
    UPROPERTY(EditDefaultsOnly, Category = "Generator")
    float SearchRadius;

    UPROPERTY(EditDefaultsOnly, Category = "Generator")
    int32 MaxPoints;

private:
    void FindCoverPoints(const FVector& Origin, TArray<FVector>& OutPoints) const;
};

// ✅ 自定义 EQS Test
UCLASS()
class MYGAME_API UEnvQueryTest_TacticalValue : public UEnvQueryTest
{
    GENERATED_BODY()

public:
    virtual void RunTest(FEnvQueryInstance& QueryInstance) const override;

protected:
    UPROPERTY(EditDefaultsOnly, Category = "Tactical")
    float CoverWeight;

    UPROPERTY(EditDefaultsOnly, Category = "Tactical")
    float VisibilityWeight;

    UPROPERTY(EditDefaultsOnly, Category = "Tactical")
    float FlankingWeight;
};

// ✅ 自定义 Behavior Tree Task
UCLASS()
class MYGAME_API UBTTask_AdvancedMoveTo : public UBTTask_BlackboardBase
{
    GENERATED_BODY()

public:
    virtual EBTNodeResult::Type ExecuteTask(UBehaviorTreeComponent& OwnerComp, uint8* NodeMemory) override;
    virtual EBTNodeResult::Type AbortTask(UBehaviorTreeComponent& OwnerComp, uint8* NodeMemory) override;
    virtual void TickTask(UBehaviorTreeComponent& OwnerComp, uint8* NodeMemory, float DeltaSeconds) override;

protected:
    virtual void OnPathUpdated(FNavigationPath* UpdatedPath);
    virtual void OnMoveFinished(FAIRequestID RequestID, EPathFollowingResult::Type Result);
};
```

---

## 🚨 资深开发者常见陷阱

### 必须避免的反模式
```cpp
// ❌ 错误：在 Tick 中频繁分配
void AMyActor::Tick(float DeltaTime)
{
    TArray<AActor*> Actors;  // 每帧分配
    UGameplayStatics::GetAllActorsOfClass(GetWorld(), AEnemy::StaticClass(), Actors);
}

// ✅ 正确：缓存查询结果或使用定时器
void AMyActor::BeginPlay()
{
    GetWorld()->GetTimerManager().SetTimer(
        CacheUpdateTimer, this, &AMyActor::UpdateEnemyCache, 0.5f, true);
}

// ❌ 错误：滥用 BlueprintImplementableEvent
UFUNCTION(BlueprintImplementableEvent)
void OnEveryFrame();  // 每帧从 C++ 调用蓝图，性能差

// ✅ 正确：使用事件驱动
UFUNCTION(BlueprintImplementableEvent)
void OnStateChanged(EMyState NewState);  // 仅状态变化时调用

// ❌ 错误：不正确的网络复制
UPROPERTY(Replicated)
TArray<FVector> PathPoints;  // 大数组每帧复制，带宽爆炸

// ✅ 正确：使用 FastArray 或 RPC
UPROPERTY()
FFastArraySerializer PathPointsSerializer;

UFUNCTION(Client, Reliable)
void ClientReceivePath(const TArray<FVector>& Path);  // 仅需要时发送
```

### 性能监控指标
| 指标 | 移动端目标 | PC端目标 | 主机目标 |
|------|-----------|---------|---------|
| 帧率 | 30 fps | 60+ fps | 60 fps |
| Draw Calls | < 500 | < 2000 | < 3000 |
| 三角面数 | < 500K | < 5M (Nanite) | < 10M (Nanite) |
| 内存使用 | < 2GB | < 8GB | < 10GB |
| Shader Complexity | < 100 | < 300 | < 500 |
| Game Thread | < 16ms | < 8ms | < 10ms |
| Render Thread | < 16ms | < 8ms | < 10ms |
| GPU Time | < 33ms | < 12ms | < 16ms |

### 内存管理黄金法则
```cpp
// ✅ 使用 UPROPERTY 管理 UObject 生命周期
UPROPERTY()
TObjectPtr<UMyObject> ManagedObject;  // GC 管理

// ✅ 使用 TWeakObjectPtr 避免循环引用
TWeakObjectPtr<AActor> WeakTarget;  // 不阻止 GC

// ✅ 使用 AddToRoot/RemoveFromRoot 管理全局对象
void UMySubsystem::Initialize(FSubsystemCollectionBase& Collection)
{
    GlobalData = NewObject<UMyData>(this);
    GlobalData->AddToRoot();  // 防止 GC
}

void UMySubsystem::Deinitialize()
{
    if (GlobalData)
    {
        GlobalData->RemoveFromRoot();
        GlobalData = nullptr;
    }
}

// ✅ 使用 FMemory 进行原始内存管理
void* RawMemory = FMemory::Malloc(Size, Alignment);
FMemory::Memzero(RawMemory, Size);
FMemory::Free(RawMemory);
```

---

## 📊 UE5 特定优化

### World Partition 大世界
```cpp
// ✅ 配置 Actor 流送
UCLASS()
class MYGAME_API AMyStreamableActor : public AActor
{
    GENERATED_BODY()

public:
    // 实现 IWorldPartitionActorLoaderInterface
    virtual bool ShouldLoadOnClient() const override { return true; }
    virtual FBox GetStreamingBounds() const override;

    // Data Layer 支持
    UPROPERTY(EditAnywhere, Category = "World Partition")
    TArray<TObjectPtr<UDataLayerAsset>> DataLayers;
};
```

### Nanite 最佳实践
- ✅ 静态几何体使用 Nanite
- ✅ 保持 Nanite Fallback Mesh 优化
- ✅ 使用 Nanite 材质限制（无 WPO、无透明）
- ✅ 配置 Nanite LOD 过渡

### Lumen 优化
- ✅ 使用 Lumen Surface Cache
- ✅ 配置 Global Illumination 质量
- ✅ 使用 Lumen Reflections
- ✅ 室内场景使用 Interior 设置

---

**Unreal 开发原则总结**：
C++ 核心 + 蓝图扩展、UE 命名规范、GAS 技能系统、服务器权威网络、Nanite/Lumen 渲染、Asset Manager 资源管理、UMG UI 分离、MetaSound 音频、自动化测试、性能分析优先、模块化 Game Features、高级网络预测、EQS 战术 AI、World Partition 大世界
