package com.metallic.chiaki.regist

import android.net.Uri
import android.util.Base64
import org.json.JSONObject
import java.net.HttpURLConnection
import java.net.URL
import java.nio.ByteBuffer
import java.nio.ByteOrder

// Mismo flujo que el cliente de escritorio (gui/src/psnaccountid.cpp): login en Sony,
// código de la URL de redirección -> token -> user_id -> Account-ID en base64 (8 bytes, little endian).
object PsnAccountIdFetcher
{
	// Constantes tomadas de gui/include/psnaccountid.h
	private const val CLIENT_ID = "ba495a24-818c-472b-b12d-ff231c1b5745"
	private const val CLIENT_SECRET = "mvaiZkRsAsI1IBkY"
	private const val TOKEN_URL = "https://auth.api.sonyentertainmentnetwork.com/2.0/oauth/token"
	private const val REDIRECT_URI = "https://remoteplay.dl.playstation.net/remoteplay/redirect"
	private const val SCOPE = "psn:clientapp referenceDataService:countryConfig.read pushNotification:webSocket.desktop.connect sessionManager:remotePlaySession.system.update"

	const val LOGIN_URL = "https://auth.api.sonyentertainmentnetwork.com/2.0/oauth/authorize?service_entity=urn:service-entity:psn&response_type=code&client_id=" + CLIENT_ID + "&redirect_uri=" + REDIRECT_URI + "&scope=" + SCOPE + "&request_locale=en_US&ui=pr&service_logo=ps&layout_type=popup&smcid=remoteplay&prompt=always&PlatformPrivacyWs1=minimal&"

	// Saca el parámetro "code" de la dirección que pega el usuario
	fun codeFromPastedUrl(text: String): String? =
		Uri.parse(text.trim()).getQueryParameter("code")?.takeIf { it.isNotEmpty() }

	// Llamadas de red bloqueantes: ejecutar fuera del hilo principal
	fun fetchAccountId(code: String): String
	{
		val auth = "Basic " + Base64.encodeToString("$CLIENT_ID:$CLIENT_SECRET".toByteArray(), Base64.NO_WRAP)

		val tokenBody = "grant_type=authorization_code&code=$code&scope=$SCOPE&redirect_uri=$REDIRECT_URI&"
		val tokenJson = request(TOKEN_URL, "POST", auth, "application/x-www-form-urlencoded", tokenBody)
		val accessToken = JSONObject(tokenJson).getString("access_token")

		val userJson = request("$TOKEN_URL/$accessToken", "GET", auth, "application/json", null)
		val userId = JSONObject(userJson).getString("user_id").toLong()

		val bytes = ByteBuffer.allocate(8).order(ByteOrder.LITTLE_ENDIAN).putLong(userId).array()
		return Base64.encodeToString(bytes, Base64.NO_WRAP)
	}

	private fun request(url: String, method: String, auth: String, contentType: String, body: String?): String
	{
		val connection = URL(url).openConnection() as HttpURLConnection
		try
		{
			connection.requestMethod = method
			connection.setRequestProperty("Authorization", auth)
			connection.setRequestProperty("Content-Type", contentType)
			connection.setRequestProperty("Accept", "application/json")
			connection.connectTimeout = 15000
			connection.readTimeout = 15000
			if(body != null)
			{
				connection.doOutput = true
				connection.outputStream.use { it.write(body.toByteArray()) }
			}
			if(connection.responseCode !in 200..299)
				throw IllegalStateException("HTTP ${connection.responseCode}")
			return connection.inputStream.bufferedReader().use { it.readText() }
		}
		finally
		{
			connection.disconnect()
		}
	}
}
