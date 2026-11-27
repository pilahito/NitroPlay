# Copia del triplet x64-windows de vcpkg + politica minima de CMake para puertos antiguos (json-c).
# vcpkg limpia las variables de entorno, por eso CMAKE_POLICY_VERSION_MINIMUM no llegaba por el workflow.
set(VCPKG_TARGET_ARCHITECTURE x64)
set(VCPKG_CRT_LINKAGE dynamic)
set(VCPKG_LIBRARY_LINKAGE dynamic)
list(APPEND VCPKG_CMAKE_CONFIGURE_OPTIONS "-DCMAKE_POLICY_VERSION_MINIMUM=3.5")
