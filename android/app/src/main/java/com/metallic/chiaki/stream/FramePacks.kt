// SPDX-License-Identifier: LicenseRef-AGPL-3.0-only-OpenSSL

package com.metallic.chiaki.stream

import android.content.Context
import android.graphics.Bitmap
import android.graphics.BitmapFactory
import org.json.JSONObject
import java.io.File

/**
 * Marco del usuario: carpeta <externalFiles>/marcos/<id>/ con frame.png (hueco transparente)
 * y theme.json opcional {"name":..,"viewport_rect":{"x","y","w","h"}} en % (o fracciones 0..1).
 * x, y, w, h quedan guardados como fracciones 0..1 de la pantalla.
 */
data class FramePack(val id: String, val name: String, val image: File,
                     val x: Float, val y: Float, val w: Float, val h: Float)

object FramePacks
{
	const val PREF_KEY = "frame_pack"
	private val VALID_ID = Regex("^[A-Za-z0-9_-]{1,64}$")
	private const val MAX_BYTES = 12L * 1024 * 1024

	fun dir(context: Context): File? =
		context.getExternalFilesDir(null)?.let { File(it, "marcos") }?.also { it.mkdirs() }

	fun load(context: Context): List<FramePack>
	{
		val base = dir(context) ?: return emptyList()
		val folders = base.listFiles { f -> f.isDirectory && VALID_ID.matches(f.name) } ?: return emptyList()
		return folders.sortedBy { it.name }.take(60).mapNotNull { folder ->
			val img = File(folder, "frame.png")
			if(!img.isFile || img.length() > MAX_BYTES)
				return@mapNotNull null
			var name = folder.name
			var x = 0f; var y = 0f; var w = 1f; var h = 1f
			try
			{
				val json = File(folder, "theme.json")
				if(json.isFile && json.length() < 65536)
				{
					val o = JSONObject(json.readText())
					o.optString("name").trim().takeIf { it.isNotEmpty() }?.let { name = it.take(60) }
					o.optJSONObject("viewport_rect")?.let { v ->
						var vx = v.optDouble("x", Double.NaN).toFloat()
						var vy = v.optDouble("y", Double.NaN).toFloat()
						var vw = v.optDouble("w", Double.NaN).toFloat()
						var vh = v.optDouble("h", Double.NaN).toFloat()
						if(!vx.isNaN() && !vy.isNaN() && !vw.isNaN() && !vh.isNaN())
						{
							// porcentajes 0..100 -> fracciones
							if(vw > 1f || vh > 1f) { vx /= 100f; vy /= 100f; vw /= 100f; vh /= 100f }
							if(vx >= 0f && vy >= 0f && vw >= 0.05f && vh >= 0.05f && vx + vw <= 1.0001f && vy + vh <= 1.0001f)
							{
								x = vx; y = vy; w = vw; h = vh
							}
						}
					}
				}
			}
			catch(e: Exception) { /* theme.json roto: vídeo a pantalla completa con el PNG encima */ }
			FramePack(folder.name, name, img, x, y, w, h)
		}
	}

	fun decode(pack: FramePack): Bitmap?
	{
		val bounds = BitmapFactory.Options().apply { inJustDecodeBounds = true }
		BitmapFactory.decodeFile(pack.image.path, bounds)
		if(bounds.outWidth <= 0 || bounds.outHeight <= 0)
			return null
		var sample = 1
		while(bounds.outWidth / sample > 3840 || bounds.outHeight / sample > 2160)
			sample *= 2
		return BitmapFactory.decodeFile(pack.image.path, BitmapFactory.Options().apply { inSampleSize = sample })
	}
}
