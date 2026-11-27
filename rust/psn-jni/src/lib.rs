//! Puente JNI hacia Kotlin: clase `com.metallic.chiaki.regist.PsnNative`.
use jni::objects::{JClass, JString};
use jni::sys::jlong;
use jni::EnvUnowned;

/// Kotlin: `@JvmStatic external fun accountIdBase64(userId: Long): String`
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_metallic_chiaki_regist_PsnNative_accountIdBase64<'caller>(
    mut unowned_env: EnvUnowned<'caller>,
    _class: JClass<'caller>,
    user_id: jlong,
) -> JString<'caller> {
    let outcome = unowned_env.with_env(|env| -> Result<_, jni::errors::Error> {
        JString::from_str(env, psn_account::account_id_base64(user_id as u64))
    });
    outcome.resolve::<jni::errors::ThrowRuntimeExAndDefault>()
}
