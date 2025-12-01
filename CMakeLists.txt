cmake_minimum_required(VERSION 3.27)

set(CMAKE_BUILD_TYPE "Release")
set(BUILD_SHARED_LIBS OFF)
set(CMAKE_OSX_DEPLOYMENT_TARGET 11.0)

set(MANIFOLD_EXPORT OFF)
set(MANIFOLD_CBIND ON)
set(MANIFOLD_TEST OFF)
add_subdirectory(vendor/manifold)
