# Keep the bundled Flutter engine and plugins out of the automatic
# Provides/Requires (system libraries such as GTK stay required), and skip
# debuginfo and .build-id links (they would clash between apps shipping the
# same engine).
%global debug_package %{nil}
%global _build_id_links none
%global __provides_exclude_from ^${INSTALL_DIR}/.*$
%global __requires_exclude ^(${RPM_PRIVATE_LIBS})

Name: ${PACKAGE_NAME}
Version: ${PACKAGE_VERSION}
Release: ${RPM_RELEASE}%{?dist}
Summary: ${APP_DESCRIPTION}
License: ${APP_LICENSE}
URL: ${APP_HOMEPAGE}
Packager: ${APP_MAINTAINER}
BuildArch: ${PACKAGE_ARCH}

%description
Hello World is the fastforge example app: a Flutter counter that shows its
build name, build number and APP_ENV.

# fastforge stages everything to install in $${PACKAGING_DIRECTORY} and lists
# it in $${RPM_FILE_LIST}.
%install
cp -a ${PACKAGING_DIRECTORY}/. %{buildroot}/

%files -f ${RPM_FILE_LIST}
