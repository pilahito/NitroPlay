package com.metallic.chiaki.regist

// Puente hacia Rust (rust/psn-jni). Si la biblioteca nativa no esta en el APK
// (otra ABI), available es false y PsnAccountIdFetcher usa el respaldo en Kotlin.
object PsnNative
{
	val available: Boolean = try
	{
		System.loadLibrary("psn_jni")
		true
	}
	catch(e: UnsatisfiedLinkError)
	{
		false
	}

	@JvmStatic external fun accountIdBase64(userId: Long): String
}
