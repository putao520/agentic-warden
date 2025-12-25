# Unreal Engine Game Development Standards - CODING-STANDARDS-UNREAL

**Version**: 2.0.0
**Scope**: Unreal Engine game development (2D/3D/Mobile/PC/Console/VR)
**Tech Stack**: UE5+, C++, Blueprints, Niagara, Lumen/Nanite
**Last Updated**: 2025-12-25

---

## 🚨 Core Iron Rules (Inherited from common.md)

> **Must follow the four core iron rules from common.md + game.md general game standards**

```
Iron Rule 1: SPEC is the Single Source of Truth (SSOT)
       - Game mechanics must comply with SPEC definitions
       - Actor, Component, DataAsset structure based on SPEC

Iron Rule 2: Smart Reuse and Destroy-Rebuild
       - Existing class fully matches → Direct reuse
       - Partial match → Delete and rebuild, no incremental modifications

Iron Rule 3: Prohibit Incremental Development
       - Prohibit adding new features to old Actor
       - Prohibit retaining deprecated UPROPERTY fields

Iron Rule 4: Context7 Research First
       - Use UE official plugins and Marketplace mature assets
       - Prohibit implementing core systems like GAS and network synchronization yourself
```

---

## 🏗️ Project Structure

### Directory Organization
```
Content/
├── _Game/                  # Project-specific resources
│   ├── Blueprints/         # Blueprint classes
│   │   ├── Core/           # Core systems
│   │   ├── Characters/     # Characters
│   │   ├── AI/             # AI behavior
│   │   └── UI/             # UMG widgets
│   ├── Maps/               # Level maps
│   ├── DataAssets/         # Data assets
│   ├── Materials/          # Materials
│   ├── Textures/           # Textures
│   ├── Meshes/             # Meshes
│   ├── Animations/         # Animations
│   ├── Audio/              # Audio
│   ├── Effects/            # VFX (Niagara)
│   └── UI/                 # UI resources
├── Plugins/                # Project plugins
└── Developers/             # Developer temporary resources (not committed)

Source/
├── MyGame/
│   ├── Public/             # Header files
│   │   ├── Core/
│   │   ├── Characters/
│   │   ├── Weapons/
│   │   └── UI/
│   ├── Private/            # Implementation files
│   └── MyGame.Build.cs     # Module configuration
└── MyGameEditor/           # Editor module
```

### Naming Conventions
- ✅ C++ Classes: Prefix identifies type
  - `A` - Actor (`AMyCharacter`)
  - `U` - UObject (`UHealthComponent`)
  - `F` - Struct/Value Type (`FDamageInfo`)
  - `E` - Enum (`EWeaponType`)
  - `I` - Interface (`IDamageable`)
  - `T` - Template (`TArray`)
- ✅ Blueprints: BP_PascalCase (`BP_Player`)
- ✅ Materials: M_PascalCase (`M_Character_Skin`)
- ✅ Textures: T_PascalCase (`T_Ground_Diffuse`)
- ❌ Prohibit spaces and Chinese naming

---

## 📜 C++ Coding Standards

### Class Declaration Structure
```cpp
// ✅ Correct class structure
UCLASS(BlueprintType, Blueprintable)
class MYGAME_API AMyCharacter : public ACharacter
{
    GENERATED_BODY()

public:
    // 1. Constructor
    AMyCharacter();

    // 2. Public methods
    UFUNCTION(BlueprintCallable, Category = "Combat")
    void TakeDamage(float Damage, AActor* DamageCauser);

protected:
    // 3. Lifecycle methods
    virtual void BeginPlay() override;
    virtual void Tick(float DeltaTime) override;

    // 4. Protected properties (Blueprint accessible)
    UPROPERTY(EditDefaultsOnly, BlueprintReadOnly, Category = "Stats")
    float MaxHealth = 100.f;

    UPROPERTY(BlueprintReadOnly, Category = "Stats")
    float CurrentHealth;

private:
    // 5. Private components
    UPROPERTY(VisibleAnywhere)
    TObjectPtr<UHealthComponent> HealthComponent;

    // 6. Private methods
    void InitializeComponents();
};
```

### UPROPERTY Specifiers
```cpp
// ✅ Common UPROPERTY configurations
UPROPERTY(EditAnywhere)           // Editable in editor
UPROPERTY(EditDefaultsOnly)       // Only default value editable
UPROPERTY(VisibleAnywhere)        // Visible but not editable in editor
UPROPERTY(BlueprintReadOnly)      // Blueprint read-only
UPROPERTY(BlueprintReadWrite)     // Blueprint read-write
UPROPERTY(Replicated)             // Network replication
UPROPERTY(ReplicatedUsing=OnRep_Health)  // Replication callback

// ✅ Combined usage
UPROPERTY(EditDefaultsOnly, BlueprintReadOnly, Category = "Combat")
float BaseDamage = 10.f;
```

### UFUNCTION Specifiers
```cpp
// ✅ Common UFUNCTION configurations
UFUNCTION(BlueprintCallable)              // Blueprint callable
UFUNCTION(BlueprintPure)                  // Pure function (no side effects)
UFUNCTION(BlueprintImplementableEvent)    // Blueprint implementable
UFUNCTION(BlueprintNativeEvent)           // C++ default implementation, Blueprint overridable
UFUNCTION(Server, Reliable)               // Server RPC
UFUNCTION(Client, Reliable)               // Client RPC
UFUNCTION(NetMulticast, Unreliable)       // Multicast RPC
```

### Smart Pointers
```cpp
// ✅ Use UE smart pointers
TObjectPtr<UObject> ObjectPtr;           // UObject pointer (UE5)
TWeakObjectPtr<AActor> WeakActor;        // Weak reference
TSharedPtr<FMyStruct> SharedStruct;      // Shared pointer (non-UObject)
TUniquePtr<FMyStruct> UniqueStruct;      // Unique pointer

// ✅ Soft references (lazy loading)
UPROPERTY(EditDefaultsOnly)
TSoftObjectPtr<UTexture2D> LazyTexture;

UPROPERTY(EditDefaultsOnly)
TSoftClassPtr<AActor> LazyActorClass;
```

---

## 🎨 Blueprint Standards

### Blueprint Organization
- ✅ Use Collapsed Nodes to organize complex logic
- ✅ Use Comments to label functional areas
- ✅ Use Reroute Nodes to organize connections
- ✅ Encapsulate complex logic in Functions/Macros
- ❌ Prohibit spaghetti blueprints

### C++ and Blueprint Collaboration
```cpp
// ✅ C++ defines core logic, Blueprints extend
UCLASS(Abstract, Blueprintable)
class MYGAME_API AWeaponBase : public AActor
{
    GENERATED_BODY()

public:
    // C++ implements core logic
    UFUNCTION(BlueprintCallable)
    void Fire();

protected:
    // Blueprint implements specific effects
    UFUNCTION(BlueprintImplementableEvent)
    void OnFire();

    // C++ default implementation, Blueprint can override
    UFUNCTION(BlueprintNativeEvent)
    void PlayFireEffect();
};
```

### Blueprint Usage Scenarios
- ✅ Rapid prototyping and iteration
- ✅ Artist/designer-tunable parameters
- ✅ Animation and VFX logic
- ✅ UI interaction logic
- ❌ Complex algorithms and performance-critical code

---

## 🌐 Network and Multiplayer

### Replication System
```cpp
// ✅ Property replication
UPROPERTY(Replicated)
float Health;

// ✅ Replication condition
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
    // Called when client receives update
    UpdateHealthUI();
}
```

### RPC Patterns
```cpp
// ✅ Server RPC (client calls, server executes)
UFUNCTION(Server, Reliable, WithValidation)
void ServerRPC_Fire();
bool ServerRPC_Fire_Validate() { return true; }
void ServerRPC_Fire_Implementation() { /* ... */ }

// ✅ Client RPC (server calls, client executes)
UFUNCTION(Client, Reliable)
void ClientRPC_ShowDamageNumber(float Damage);

// ✅ Multicast RPC (server calls, all clients execute)
UFUNCTION(NetMulticast, Unreliable)
void MulticastRPC_PlayExplosion();
```

### Network Authority
- ✅ Server-authoritative mode
- ✅ Client prediction + server validation
- ✅ Use `HasAuthority()` to check permissions
- ❌ Prohibit client direct modification of replicated properties

---

## ⚔️ Gameplay Ability System (GAS)

### Core Concepts
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
    // Configure damage, duration, modifiers, etc.
};
```

### GAS Best Practices
- ✅ Use Gameplay Tags to manage state
- ✅ Use Gameplay Effects for attribute modification
- ✅ Use Gameplay Cues for VFX
- ✅ Use Target Data to pass target information
- ❌ Prohibit bypassing GAS to directly modify attributes

---

## 🎨 Rendering and Performance

### UE5 Features
- ✅ Nanite virtual geometry (static meshes)
- ✅ Lumen global illumination
- ✅ Virtual Shadow Maps
- ✅ World Partition open world streaming

### Performance Optimization
```cpp
// ✅ Use Stat commands for analysis
DECLARE_STATS_GROUP(TEXT("MyGame"), STATGROUP_MyGame, STATCAT_Advanced);
DECLARE_CYCLE_STAT(TEXT("Update Combat"), STAT_UpdateCombat, STATGROUP_MyGame);

void AMyCharacter::UpdateCombat()
{
    SCOPE_CYCLE_COUNTER(STAT_UpdateCombat);
    // ...
}
```

### Memory Management
- ✅ Use Asset Manager for resource management
- ✅ Configure Primary Asset Types
- ✅ Use Soft References for lazy loading
- ✅ Use Streaming Levels

### Mobile Optimization
- ✅ Use Mobile Forward Renderer
- ✅ Texture compression (ASTC/ETC2)
- ✅ Reduce Draw Calls
- ✅ Disable advanced rendering features

---

## 🖼️ UI System (UMG)

### Widget Structure
```cpp
// ✅ C++ Widget base class
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

### UI Best Practices
- ✅ C++ defines logic, Blueprints define layout
- ✅ Use `meta = (BindWidget)` to bind widgets
- ✅ Use Common UI plugin (gamepad support)
- ✅ Use Widget Component for 3D UI
- ❌ Prohibit updating static UI in Tick

---

## 🎵 Audio System

### MetaSound (UE5)
- ✅ Use MetaSound for procedural audio
- ✅ Use Sound Classes for volume management
- ✅ Use Sound Attenuation for 3D audio
- ✅ Use Audio Modulation for dynamic modulation

### Audio Optimization
- ✅ Use Sound Concurrency to limit concurrent sounds
- ✅ Stream long music
- ✅ Use audio pooling
- ❌ Avoid playing too many sounds simultaneously

---

## 🧪 Testing

### Automation Testing
```cpp
// ✅ Automation tests
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

### Test Types
- ✅ Unit Tests (C++ logic tests)
- ✅ Functional Tests (in-game tests)
- ✅ Screenshot Tests (visual regression tests)
- ✅ Gauntlet (automated performance tests)

---

## 📋 Unreal Development Checklist

### Code Quality
- [ ] Follow UE naming conventions (A/U/F/E/I prefixes)
- [ ] Correct use of UPROPERTY/UFUNCTION specifiers
- [ ] Use TObjectPtr and smart pointers
- [ ] C++ core logic + Blueprint extension

### Network
- [ ] Property replication configured correctly
- [ ] RPC permission validation
- [ ] Server-authoritative mode
- [ ] Network performance optimization

### Performance
- [ ] Use Stat commands for analysis
- [ ] Resource streaming configuration
- [ ] Draw Call optimization
- [ ] Mobile adaptation

### Architecture
- [ ] GAS ability system (complex combat)
- [ ] UI logic separation
- [ ] Data Assets data-driven
- [ ] Automation test coverage

---

## 🏛️ Advanced Architecture Patterns

### Modular Game Framework (Modular Game Features)
```cpp
// ✅ Use Game Features and Modular Gameplay plugins
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

// ✅ Manage module state via GameplayTags
namespace MyGameTags
{
    UE_DEFINE_GAMEPLAY_TAG(Feature_Combat, "Feature.Combat");
    UE_DEFINE_GAMEPLAY_TAG(Feature_Stealth, "Feature.Stealth");
    UE_DEFINE_GAMEPLAY_TAG(Feature_Vehicle, "Feature.Vehicle");
}
```

### Advanced GAS Architecture
```cpp
// ✅ Attribute Set organization
UCLASS()
class MYGAME_API UMyAttributeSet : public UAttributeSet
{
    GENERATED_BODY()

public:
    // Use macros to simplify property definitions
    ATTRIBUTE_ACCESSORS(UMyAttributeSet, Health);
    ATTRIBUTE_ACCESSORS(UMyAttributeSet, MaxHealth);
    ATTRIBUTE_ACCESSORS(UMyAttributeSet, Damage);

    // Intercept before attribute change
    virtual void PreAttributeChange(const FGameplayAttribute& Attribute, float& NewValue) override;

    // Handle after attribute change
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

### Advanced Network Prediction
```cpp
// ✅ Custom movement prediction
UCLASS()
class MYGAME_API UMyCharacterMovementComponent : public UCharacterMovementComponent
{
    GENERATED_BODY()

public:
    // Custom movement mode
    virtual void PhysCustom(float DeltaTime, int32 Iterations) override;

    // Client prediction
    virtual void MoveAutonomous(
        float ClientTimeStamp,
        float DeltaTime,
        uint8 CompressedFlags,
        const FVector& NewAccel) override;

    // Server correction
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
    // Save movement state for rollback
    virtual void SaveMoveState();
    virtual void RestoreMoveState();
};

// ✅ Gameplay Prediction Key
void AMyCharacter::PerformAbility()
{
    if (HasAuthority())
    {
        // Server executes directly
        ExecuteAbility();
    }
    else
    {
        // Client prediction
        FScopedPredictionWindow ScopedPrediction(AbilitySystemComponent);
        FPredictionKey PredictionKey = AbilitySystemComponent->GetPredictionKeyForNewAction();

        ExecuteAbility_Predicted(PredictionKey);
        ServerExecuteAbility(PredictionKey);
    }
}
```

### Plugin Architecture Design
```cpp
// ✅ Modular plugin structure
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

// ✅ Module interface
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

## 🔧 Essential Skills for Senior Developers

### Custom Slate UI
```cpp
// ✅ High-performance custom Slate widget
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

### Advanced Material System
```cpp
// ✅ Runtime material instance management
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

    // Use Material Parameter Collection for batch updates
    UPROPERTY()
    TObjectPtr<UMaterialParameterCollection> GlobalMPC;
};

// ✅ Procedural material generation
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

### Niagara Advanced VFX
```cpp
// ✅ Procedural Niagara system
UCLASS()
class MYGAME_API UNiagaraEffectManager : public UObject
{
    GENERATED_BODY()

public:
    void SpawnEffect(UNiagaraSystem* System, const FTransform& Transform, const FNiagaraEffectParams& Params);
    void UpdateEffectParameter(UNiagaraComponent* Component, FName ParameterName, const FVector& Value);

    // Object pooling Niagara components
    UNiagaraComponent* AcquireComponent(UNiagaraSystem* System);
    void ReleaseComponent(UNiagaraComponent* Component);

private:
    TMap<UNiagaraSystem*, TArray<TObjectPtr<UNiagaraComponent>>> ComponentPools;
};

// ✅ Data Interface custom data source
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

### Advanced AI System (EQS + Behavior Tree)
```cpp
// ✅ Custom EQS Generator
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

// ✅ Custom EQS Test
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

// ✅ Custom Behavior Tree Task
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

## 🚨 Common Pitfalls for Senior Developers

### Must Avoid Anti-Patterns
```cpp
// ❌ Wrong: Frequent allocation in Tick
void AMyActor::Tick(float DeltaTime)
{
    TArray<AActor*> Actors;  // Allocates every frame
    UGameplayStatics::GetAllActorsOfClass(GetWorld(), AEnemy::StaticClass(), Actors);
}

// ✅ Correct: Cache query results or use timers
void AMyActor::BeginPlay()
{
    GetWorld()->GetTimerManager().SetTimer(
        CacheUpdateTimer, this, &AMyActor::UpdateEnemyCache, 0.5f, true);
}

// ❌ Wrong: Abuse of BlueprintImplementableEvent
UFUNCTION(BlueprintImplementableEvent)
void OnEveryFrame();  // Call Blueprint from C++ every frame, poor performance

// ✅ Correct: Use event-driven
UFUNCTION(BlueprintImplementableEvent)
void OnStateChanged(EMyState NewState);  // Call only when state changes

// ❌ Wrong: Improper network replication
UPROPERTY(Replicated)
TArray<FVector> PathPoints;  // Large array replicated every frame, bandwidth explosion

// ✅ Correct: Use FastArray or RPC
UPROPERTY()
FFastArraySerializer PathPointsSerializer;

UFUNCTION(Client, Reliable)
void ClientReceivePath(const TArray<FVector>& Path);  // Send only when needed
```

### Performance Monitoring Metrics
| Metric | Mobile Target | PC Target | Console Target |
|--------|--------------|-----------|----------------|
| Frame Rate | 30 fps | 60+ fps | 60 fps |
| Draw Calls | < 500 | < 2000 | < 3000 |
| Triangles | < 500K | < 5M (Nanite) | < 10M (Nanite) |
| Memory Usage | < 2GB | < 8GB | < 10GB |
| Shader Complexity | < 100 | < 300 | < 500 |
| Game Thread | < 16ms | < 8ms | < 10ms |
| Render Thread | < 16ms | < 8ms | < 10ms |
| GPU Time | < 33ms | < 12ms | < 16ms |

### Memory Management Golden Rules
```cpp
// ✅ Use UPROPERTY to manage UObject lifecycle
UPROPERTY()
TObjectPtr<UMyObject> ManagedObject;  // GC managed

// ✅ Use TWeakObjectPtr to avoid circular references
TWeakObjectPtr<AActor> WeakTarget;  // Does not prevent GC

// ✅ Use AddToRoot/RemoveFromRoot for global object management
void UMySubsystem::Initialize(FSubsystemCollectionBase& Collection)
{
    GlobalData = NewObject<UMyData>(this);
    GlobalData->AddToRoot();  // Prevent GC
}

void UMySubsystem::Deinitialize()
{
    if (GlobalData)
    {
        GlobalData->RemoveFromRoot();
        GlobalData = nullptr;
    }
}

// ✅ Use FMemory for raw memory management
void* RawMemory = FMemory::Malloc(Size, Alignment);
FMemory::Memzero(RawMemory, Size);
FMemory::Free(RawMemory);
```

---

## 📊 UE5-Specific Optimization

### World Partition Open World
```cpp
// ✅ Configure Actor streaming
UCLASS()
class MYGAME_API AMyStreamableActor : public AActor
{
    GENERATED_BODY()

public:
    // Implement IWorldPartitionActorLoaderInterface
    virtual bool ShouldLoadOnClient() const override { return true; }
    virtual FBox GetStreamingBounds() const override;

    // Data Layer support
    UPROPERTY(EditAnywhere, Category = "World Partition")
    TArray<TObjectPtr<UDataLayerAsset>> DataLayers;
};
```

### Nanite Best Practices
- ✅ Use Nanite for static geometry
- ✅ Keep Nanite Fallback Mesh optimized
- ✅ Use Nanite material restrictions (no WPO, no transparency)
- ✅ Configure Nanite LOD transitions

### Lumen Optimization
- ✅ Use Lumen Surface Cache
- ✅ Configure Global Illumination quality
- ✅ Use Lumen Reflections
- ✅ Use Interior settings for indoor scenes

---

**Unreal Development Principles Summary**:
C++ Core + Blueprint Extension, UE Naming Conventions, GAS Ability System, Server-Authoritative Network, Nanite/Lumen Rendering, Asset Manager Resource Management, UMG UI Separation, MetaSound Audio, Automation Testing, Performance Analysis First, Modular Game Features, Advanced Network Prediction, EQS Tactical AI, World Partition Open World
