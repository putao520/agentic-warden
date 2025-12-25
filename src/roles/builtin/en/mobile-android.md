# Android Application Development Standards - CODING-STANDARDS-ANDROID

**Version**: 2.0.0
**Scope**: Android native application development (Kotlin/Java, Jetpack, NDK)
**Tech Stack**: Kotlin, Jetpack Compose, Coroutines, Hilt, Room, Retrofit
**Last Updated**: 2025-12-25

---

## 🚨 Core Iron Rules (Inherited from common.md)

> **Must follow the four core iron rules from common.md**

```
Iron Rule 1: SPEC is the Single Source of Truth (SSOT)
       - UI/UX implementation must comply with SPEC definitions
       - Data models, API interfaces based on SPEC

Iron Rule 2: Smart Reuse and Destroy-Rebuild
       - Existing component fully matches → Direct reuse
       - Partial match → Delete and rebuild, no incremental modifications

Iron Rule 3: Prohibit Incremental Development
       - Prohibit adding new features to old Activity/Fragment
       - Prohibit retaining deprecated XML layouts and resources

Iron Rule 4: Context7 Research First
       - Use Jetpack official libraries and mature third-party libraries
       - Prohibit implementing infrastructure like networking and image loading yourself
```

---

## 🏗️ Project Architecture

### Clean Architecture + MVVM
```
app/
├── data/                    # Data layer
│   ├── local/               # Local data sources
│   │   ├── dao/             # Room DAO
│   │   └── entity/          # Database entities
│   ├── remote/              # Remote data sources
│   │   ├── api/             # Retrofit API
│   │   └── dto/             # Data transfer objects
│   └── repository/          # Repository implementations
├── domain/                  # Domain layer
│   ├── model/               # Domain models
│   ├── repository/          # Repository interfaces
│   └── usecase/             # Use cases
├── presentation/            # Presentation layer
│   ├── ui/                  # Compose UI
│   │   ├── screens/         # Screens
│   │   ├── components/      # Reusable components
│   │   └── theme/           # Theme
│   └── viewmodel/           # ViewModel
└── di/                      # Dependency injection modules
```

### Modular Architecture
```kotlin
// ✅ Multi-module project structure
// settings.gradle.kts
include(":app")
include(":core:common")
include(":core:network")
include(":core:database")
include(":core:ui")
include(":feature:home")
include(":feature:profile")
include(":feature:settings")

// ✅ Module dependency rules
// feature modules can only depend on core modules
// app module depends on all feature modules
// core modules尽量 independent
```

---

## 📜 Kotlin Coding Standards

### Coroutines Best Practices
```kotlin
// ✅ Use viewModelScope in ViewModel
class UserViewModel @Inject constructor(
    private val getUserUseCase: GetUserUseCase
) : ViewModel() {

    private val _uiState = MutableStateFlow<UserUiState>(UserUiState.Loading)
    val uiState: StateFlow<UserUiState> = _uiState.asStateFlow()

    fun loadUser(userId: String) {
        viewModelScope.launch {
            _uiState.value = UserUiState.Loading
            getUserUseCase(userId)
                .catch { e -> _uiState.value = UserUiState.Error(e.message) }
                .collect { user -> _uiState.value = UserUiState.Success(user) }
        }
    }
}

// ✅ Use Dispatchers in Repository
class UserRepositoryImpl @Inject constructor(
    private val api: UserApi,
    private val dao: UserDao,
    @IoDispatcher private val ioDispatcher: CoroutineDispatcher
) : UserRepository {

    override fun getUser(id: String): Flow<User> = flow {
        // First get from local
        dao.getUser(id)?.let { emit(it.toDomain()) }

        // Then update from network
        val remoteUser = api.getUser(id)
        dao.insertUser(remoteUser.toEntity())
        emit(remoteUser.toDomain())
    }.flowOn(ioDispatcher)
}

// ✅ Structured concurrency
suspend fun fetchDataConcurrently() = coroutineScope {
    val users = async { userRepository.getUsers() }
    val posts = async { postRepository.getPosts() }
    CombinedData(users.await(), posts.await())
}
```

### Flow Advanced Usage
```kotlin
// ✅ StateFlow + SharedFlow
class SearchViewModel @Inject constructor(
    private val searchUseCase: SearchUseCase
) : ViewModel() {

    private val searchQuery = MutableStateFlow("")

    val searchResults: StateFlow<List<SearchResult>> = searchQuery
        .debounce(300)
        .filter { it.length >= 2 }
        .distinctUntilChanged()
        .flatMapLatest { query ->
            searchUseCase(query)
                .catch { emit(emptyList()) }
        }
        .stateIn(
            scope = viewModelScope,
            started = SharingStarted.WhileSubscribed(5000),
            initialValue = emptyList()
        )

    fun onQueryChanged(query: String) {
        searchQuery.value = query
    }
}

// ✅ Channel for one-time events
private val _events = Channel<UiEvent>(Channel.BUFFERED)
val events: Flow<UiEvent> = _events.receiveAsFlow()

fun showSnackbar(message: String) {
    viewModelScope.launch {
        _events.send(UiEvent.ShowSnackbar(message))
    }
}
```

---

## 🎨 Jetpack Compose

### Compose Architecture
```kotlin
// ✅ State lifting + unidirectional data flow
@Composable
fun UserScreen(
    viewModel: UserViewModel = hiltViewModel()
) {
    val uiState by viewModel.uiState.collectAsStateWithLifecycle()

    UserContent(
        uiState = uiState,
        onRefresh = viewModel::refresh,
        onItemClick = viewModel::onItemClick
    )
}

// ✅ Stateless composable (testable)
@Composable
fun UserContent(
    uiState: UserUiState,
    onRefresh: () -> Unit,
    onItemClick: (User) -> Unit,
    modifier: Modifier = Modifier
) {
    when (uiState) {
        is UserUiState.Loading -> LoadingIndicator()
        is UserUiState.Success -> UserList(
            users = uiState.users,
            onItemClick = onItemClick
        )
        is UserUiState.Error -> ErrorMessage(
            message = uiState.message,
            onRetry = onRefresh
        )
    }
}
```

### Performance Optimization
```kotlin
// ✅ Use remember and derivedStateOf
@Composable
fun ItemList(items: List<Item>) {
    val sortedItems by remember(items) {
        derivedStateOf { items.sortedBy { it.name } }
    }

    LazyColumn {
        items(
            items = sortedItems,
            key = { it.id }  // ✅ Use key to optimize recomposition
        ) { item ->
            ItemRow(item = item)
        }
    }
}

// ✅ Use Immutable annotation
@Immutable
data class UserUiModel(
    val id: String,
    val name: String,
    val avatarUrl: String
)

// ✅ Use Stable annotation for callbacks
@Stable
class UserListCallbacks(
    val onItemClick: (User) -> Unit,
    val onItemLongClick: (User) -> Unit,
    val onDeleteClick: (User) -> Unit
)
```

### Custom Compose Components
```kotlin
// ✅ Custom Modifier
fun Modifier.shimmerEffect(): Modifier = composed {
    var size by remember { mutableStateOf(IntSize.Zero) }
    val transition = rememberInfiniteTransition(label = "shimmer")
    val startOffsetX by transition.animateFloat(
        initialValue = -2 * size.width.toFloat(),
        targetValue = 2 * size.width.toFloat(),
        animationSpec = infiniteRepeatable(
            animation = tween(1000)
        ),
        label = "shimmerOffset"
    )

    background(
        brush = Brush.linearGradient(
            colors = listOf(
                Color.LightGray.copy(alpha = 0.6f),
                Color.LightGray.copy(alpha = 0.2f),
                Color.LightGray.copy(alpha = 0.6f)
            ),
            start = Offset(startOffsetX, 0f),
            end = Offset(startOffsetX + size.width, size.height.toFloat())
        )
    ).onGloballyPositioned { size = it.size }
}

// ✅ Custom Layout
@Composable
fun FlowRow(
    modifier: Modifier = Modifier,
    horizontalSpacing: Dp = 8.dp,
    verticalSpacing: Dp = 8.dp,
    content: @Composable () -> Unit
) {
    Layout(
        content = content,
        modifier = modifier
    ) { measurables, constraints ->
        // Measure and layout logic
        layout(width, height) {
            // Place children
        }
    }
}
```

---

## 💉 Dependency Injection (Hilt)

### Hilt Module Organization
```kotlin
// ✅ Network module
@Module
@InstallIn(SingletonComponent::class)
object NetworkModule {

    @Provides
    @Singleton
    fun provideOkHttpClient(
        authInterceptor: AuthInterceptor,
        loggingInterceptor: HttpLoggingInterceptor
    ): OkHttpClient = OkHttpClient.Builder()
        .addInterceptor(authInterceptor)
        .addInterceptor(loggingInterceptor)
        .connectTimeout(30, TimeUnit.SECONDS)
        .readTimeout(30, TimeUnit.SECONDS)
        .build()

    @Provides
    @Singleton
    fun provideRetrofit(
        okHttpClient: OkHttpClient,
        json: Json
    ): Retrofit = Retrofit.Builder()
        .baseUrl(BuildConfig.API_BASE_URL)
        .client(okHttpClient)
        .addConverterFactory(json.asConverterFactory("application/json".toMediaType()))
        .build()
}

// ✅ Dispatcher module
@Module
@InstallIn(SingletonComponent::class)
object DispatcherModule {

    @IoDispatcher
    @Provides
    fun provideIoDispatcher(): CoroutineDispatcher = Dispatchers.IO

    @DefaultDispatcher
    @Provides
    fun provideDefaultDispatcher(): CoroutineDispatcher = Dispatchers.Default

    @MainDispatcher
    @Provides
    fun provideMainDispatcher(): CoroutineDispatcher = Dispatchers.Main
}

@Qualifier
@Retention(AnnotationRetention.BINARY)
annotation class IoDispatcher

@Qualifier
@Retention(AnnotationRetention.BINARY)
annotation class DefaultDispatcher

@Qualifier
@Retention(AnnotationRetention.BINARY)
annotation class MainDispatcher
```

---

## 🗄️ Data Persistence (Room)

### Room Best Practices
```kotlin
// ✅ Entity design
@Entity(
    tableName = "users",
    indices = [Index(value = ["email"], unique = true)]
)
data class UserEntity(
    @PrimaryKey
    val id: String,
    val name: String,
    val email: String,
    @ColumnInfo(name = "avatar_url")
    val avatarUrl: String?,
    @ColumnInfo(name = "created_at")
    val createdAt: Long = System.currentTimeMillis()
)

// ✅ DAO with Flow
@Dao
interface UserDao {
    @Query("SELECT * FROM users ORDER BY created_at DESC")
    fun observeUsers(): Flow<List<UserEntity>>

    @Query("SELECT * FROM users WHERE id = :id")
    suspend fun getUser(id: String): UserEntity?

    @Insert(onConflict = OnConflictStrategy.REPLACE)
    suspend fun insertUsers(users: List<UserEntity>)

    @Transaction
    suspend fun replaceAllUsers(users: List<UserEntity>) {
        deleteAll()
        insertUsers(users)
    }

    @Query("DELETE FROM users")
    suspend fun deleteAll()
}

// ✅ Database with TypeConverter
@Database(
    entities = [UserEntity::class, PostEntity::class],
    version = 1,
    exportSchema = true
)
@TypeConverters(Converters::class)
abstract class AppDatabase : RoomDatabase() {
    abstract fun userDao(): UserDao
    abstract fun postDao(): PostDao
}

class Converters {
    @TypeConverter
    fun fromTimestamp(value: Long?): Date? = value?.let { Date(it) }

    @TypeConverter
    fun dateToTimestamp(date: Date?): Long? = date?.time
}
```

---

## 🌐 Network Layer (Retrofit)

### Retrofit Configuration
```kotlin
// ✅ API interface definition
interface UserApi {
    @GET("users")
    suspend fun getUsers(
        @Query("page") page: Int,
        @Query("limit") limit: Int = 20
    ): Response<List<UserDto>>

    @GET("users/{id}")
    suspend fun getUser(@Path("id") id: String): Response<UserDto>

    @POST("users")
    suspend fun createUser(@Body user: CreateUserRequest): Response<UserDto>

    @Multipart
    @POST("users/{id}/avatar")
    suspend fun uploadAvatar(
        @Path("id") id: String,
        @Part image: MultipartBody.Part
    ): Response<AvatarResponse>
}

// ✅ Network result wrapper
sealed class NetworkResult<out T> {
    data class Success<T>(val data: T) : NetworkResult<T>()
    data class Error(val code: Int, val message: String) : NetworkResult<Nothing>()
    data class Exception(val throwable: Throwable) : NetworkResult<Nothing>()
}

// ✅ Safe API call
suspend fun <T> safeApiCall(
    apiCall: suspend () -> Response<T>
): NetworkResult<T> = try {
    val response = apiCall()
    if (response.isSuccessful) {
        response.body()?.let { NetworkResult.Success(it) }
            ?: NetworkResult.Error(response.code(), "Empty body")
    } else {
        NetworkResult.Error(response.code(), response.message())
    }
} catch (e: IOException) {
    NetworkResult.Exception(e)
} catch (e: HttpException) {
    NetworkResult.Error(e.code(), e.message())
}
```

---

## 🔒 Security Best Practices

### Data Encryption
```kotlin
// ✅ EncryptedSharedPreferences
@Provides
@Singleton
fun provideEncryptedPrefs(@ApplicationContext context: Context): SharedPreferences {
    val masterKey = MasterKey.Builder(context)
        .setKeyScheme(MasterKey.KeyScheme.AES256_GCM)
        .build()

    return EncryptedSharedPreferences.create(
        context,
        "secure_prefs",
        masterKey,
        EncryptedSharedPreferences.PrefKeyEncryptionScheme.AES256_SIV,
        EncryptedSharedPreferences.PrefValueEncryptionScheme.AES256_GCM
    )
}

// ✅ Keystore key management
class KeystoreManager {
    private val keyStore = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }

    fun getOrCreateSecretKey(alias: String): SecretKey {
        keyStore.getEntry(alias, null)?.let {
            return (it as KeyStore.SecretKeyEntry).secretKey
        }

        val keyGenerator = KeyGenerator.getInstance(
            KeyProperties.KEY_ALGORITHM_AES,
            "AndroidKeyStore"
        )
        keyGenerator.init(
            KeyGenParameterSpec.Builder(
                alias,
                KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT
            )
                .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
                .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
                .setUserAuthenticationRequired(false)
                .build()
        )
        return keyGenerator.generateKey()
    }
}
```

### ProGuard/R8 Configuration
```proguard
# ✅ Must keep rules
-keepattributes *Annotation*
-keepattributes Signature
-keepattributes SourceFile,LineNumberTable

# Kotlin Serialization
-keepattributes InnerClasses
-keep class kotlinx.serialization.** { *; }
-keepclassmembers class * {
    @kotlinx.serialization.SerialName <fields>;
}

# Retrofit
-keepclassmembers,allowshrinking,allowobfuscation interface * {
    @retrofit2.http.* <methods>;
}

# Room
-keep class * extends androidx.room.RoomDatabase
-keep @androidx.room.Entity class *
```

---

## ⚡ Performance Optimization

### Startup Optimization
```kotlin
// ✅ App Startup library
class MyInitializer : Initializer<MyDependency> {
    override fun create(context: Context): MyDependency {
        // Delay initialization of non-critical dependencies
        return MyDependency()
    }

    override fun dependencies(): List<Class<out Initializer<*>>> {
        return listOf(WorkManagerInitializer::class.java)
    }
}

// ✅ Baseline Profiles
// Use Macrobenchmark to generate Baseline Profile
@OptIn(ExperimentalBaselineProfilesApi::class)
@RunWith(AndroidJUnit4::class)
class BaselineProfileGenerator {
    @get:Rule
    val baselineProfileRule = BaselineProfileRule()

    @Test
    fun generate() = baselineProfileRule.collect(packageName = "com.example.app") {
        startActivityAndWait()
        // Execute critical user paths
    }
}
```

### Memory Optimization
```kotlin
// ✅ Avoid memory leaks
class MyFragment : Fragment() {
    private var _binding: FragmentMyBinding? = null
    private val binding get() = _binding!!

    override fun onDestroyView() {
        super.onDestroyView()
        _binding = null  // Must clear
    }
}

// ✅ WeakReference callback
class NetworkCallback(fragment: MyFragment) {
    private val fragmentRef = WeakReference(fragment)

    fun onSuccess(data: Data) {
        fragmentRef.get()?.handleSuccess(data)
    }
}

// ✅ LeakCanary integration
debugImplementation("com.squareup.leakcanary:leakcanary-android:2.12")
```

### Performance Monitoring Metrics
| Metric | Target |
|--------|--------|
| Cold Start Time | < 2s |
| Warm Start Time | < 500ms |
| Frame Rate | 60 fps |
| Frame Drop Rate | < 1% |
| ANR Rate | < 0.1% |
| Crash Rate | < 0.1% |
| Memory Usage | < 200MB |
| APK Size | < 50MB |

---

## 🧪 Testing

### Unit Tests
```kotlin
// ✅ ViewModel testing
@OptIn(ExperimentalCoroutinesApi::class)
class UserViewModelTest {
    @get:Rule
    val mainDispatcherRule = MainDispatcherRule()

    private lateinit var viewModel: UserViewModel
    private val getUserUseCase: GetUserUseCase = mockk()

    @Before
    fun setup() {
        viewModel = UserViewModel(getUserUseCase)
    }

    @Test
    fun `loadUser success updates state`() = runTest {
        val user = User("1", "Test User")
        coEvery { getUserUseCase("1") } returns flowOf(user)

        viewModel.loadUser("1")

        assertEquals(UserUiState.Success(user), viewModel.uiState.value)
    }
}

// ✅ Repository testing
@OptIn(ExperimentalCoroutinesApi::class)
class UserRepositoryTest {
    private lateinit var repository: UserRepository
    private val api: UserApi = mockk()
    private val dao: UserDao = mockk()

    @Test
    fun `getUser returns cached then remote`() = runTest {
        val cachedUser = UserEntity("1", "Cached")
        val remoteUser = UserDto("1", "Remote")

        coEvery { dao.getUser("1") } returns cachedUser
        coEvery { api.getUser("1") } returns Response.success(remoteUser)
        coEvery { dao.insertUser(any()) } just Runs

        val results = repository.getUser("1").toList()

        assertEquals(2, results.size)
        assertEquals("Cached", results[0].name)
        assertEquals("Remote", results[1].name)
    }
}
```

### UI Testing (Compose)
```kotlin
// ✅ Compose UI testing
class UserScreenTest {
    @get:Rule
    val composeTestRule = createComposeRule()

    @Test
    fun `shows loading indicator when loading`() {
        composeTestRule.setContent {
            UserContent(uiState = UserUiState.Loading, onRefresh = {}, onItemClick = {})
        }

        composeTestRule
            .onNodeWithContentDescription("Loading")
            .assertIsDisplayed()
    }

    @Test
    fun `shows user list when success`() {
        val users = listOf(User("1", "Test User"))

        composeTestRule.setContent {
            UserContent(
                uiState = UserUiState.Success(users),
                onRefresh = {},
                onItemClick = {}
            )
        }

        composeTestRule
            .onNodeWithText("Test User")
            .assertIsDisplayed()
    }
}
```

---

## 📋 Android Development Checklist

### Architecture
- [ ] Clean Architecture + MVVM
- [ ] Modular project structure
- [ ] Dependency injection (Hilt)
- [ ] Unidirectional data flow

### Code Quality
- [ ] Kotlin Coroutines + Flow
- [ ] State lifting Compose
- [ ] Use Immutable data classes
- [ ] Avoid memory leaks

### Performance
- [ ] Cold start < 2s
- [ ] 60 fps smooth
- [ ] Baseline Profiles
- [ ] R8 code obfuscation

### Security
- [ ] EncryptedSharedPreferences
- [ ] Keystore key management
- [ ] ProGuard rules
- [ ] Certificate pinning

### Testing
- [ ] ViewModel unit tests
- [ ] Repository tests
- [ ] Compose UI tests
- [ ] End-to-end tests

---

**Android Development Principles Summary**:
Clean Architecture, Kotlin Coroutines, Jetpack Compose, Hilt Dependency Injection, Room Database, Retrofit Network, Secure Storage, Performance Optimization, Unit Testing, Modular Architecture
