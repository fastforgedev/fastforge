#[allow(unused_imports)]
use progenitor_client::{encode_path, ClientHooks, OperationInfo, RequestBuilderExt};
#[allow(unused_imports)]
pub use progenitor_client::{ByteStream, ClientInfo, Error, ResponseValue};
/// Types used as operation parameters and responses.
#[allow(clippy::all)]
pub mod types {
    /// Error types.
    pub mod error {
        /// Error from a `TryFrom` or `FromStr` implementation.
        pub struct ConversionError(::std::borrow::Cow<'static, str>);
        impl ::std::error::Error for ConversionError {}
        impl ::std::fmt::Display for ConversionError {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> Result<(), ::std::fmt::Error> {
                ::std::fmt::Display::fmt(&self.0, f)
            }
        }

        impl ::std::fmt::Debug for ConversionError {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> Result<(), ::std::fmt::Error> {
                ::std::fmt::Debug::fmt(&self.0, f)
            }
        }

        impl From<&'static str> for ConversionError {
            fn from(value: &'static str) -> Self {
                Self(value.into())
            }
        }

        impl From<String> for ConversionError {
            fn from(value: String) -> Self {
                Self(value.into())
            }
        }
    }

    ///`AppIdsResponse`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "allOf": [
    ///    {
    ///      "$ref": "#/components/schemas/ResultResponse"
    ///    },
    ///    {
    ///      "type": "object",
    ///      "properties": {
    ///        "appids": {
    ///          "type": "array",
    ///          "items": {
    ///            "$ref": "#/components/schemas/KeyValue"
    ///          }
    ///        }
    ///      }
    ///    }
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct AppIdsResponse {
        #[serde(default, skip_serializing_if = "::std::vec::Vec::is_empty")]
        pub appids: ::std::vec::Vec<KeyValue>,
        pub ret: Result,
    }

    ///`AppInfo`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "properties": {
    ///    "appNetType": {
    ///      "type": "integer",
    ///      "format": "int32"
    ///    },
    ///    "childType": {
    ///      "type": "integer",
    ///      "format": "int32"
    ///    },
    ///    "contentRate": {
    ///      "type": "string"
    ///    },
    ///    "defaultLang": {
    ///      "type": "string"
    ///    },
    ///    "deviceTypes": {
    ///      "type": "array",
    ///      "items": {
    ///        "$ref": "#/components/schemas/DeviceTypeInfo"
    ///      }
    ///    },
    ///    "grandChildType": {
    ///      "type": "integer",
    ///      "format": "int32"
    ///    },
    ///    "isFree": {
    ///      "type": "integer",
    ///      "format": "int32"
    ///    },
    ///    "onShelfVersionCode": {
    ///      "type": "integer",
    ///      "format": "int64"
    ///    },
    ///    "onShelfVersionId": {
    ///      "type": "string"
    ///    },
    ///    "onShelfVersionNumber": {
    ///      "type": "string"
    ///    },
    ///    "parentType": {
    ///      "type": "integer",
    ///      "format": "int32"
    ///    },
    ///    "price": {
    ///      "type": "string"
    ///    },
    ///    "priceDetail": {
    ///      "type": "string"
    ///    },
    ///    "privacyPolicy": {
    ///      "type": "string"
    ///    },
    ///    "publishCountry": {
    ///      "type": "string"
    ///    },
    ///    "releaseState": {
    ///      "type": "integer",
    ///      "format": "int32"
    ///    },
    ///    "shareLink": {
    ///      "type": "string"
    ///    },
    ///    "versionCode": {
    ///      "type": "integer",
    ///      "format": "int64"
    ///    },
    ///    "versionId": {
    ///      "type": "string"
    ///    },
    ///    "versionNumber": {
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct AppInfo {
        #[serde(
            rename = "appNetType",
            default,
            skip_serializing_if = "::std::option::Option::is_none"
        )]
        pub app_net_type: ::std::option::Option<i32>,
        #[serde(
            rename = "childType",
            default,
            skip_serializing_if = "::std::option::Option::is_none"
        )]
        pub child_type: ::std::option::Option<i32>,
        #[serde(
            rename = "contentRate",
            default,
            skip_serializing_if = "::std::option::Option::is_none"
        )]
        pub content_rate: ::std::option::Option<::std::string::String>,
        #[serde(
            rename = "defaultLang",
            default,
            skip_serializing_if = "::std::option::Option::is_none"
        )]
        pub default_lang: ::std::option::Option<::std::string::String>,
        #[serde(
            rename = "deviceTypes",
            default,
            skip_serializing_if = "::std::vec::Vec::is_empty"
        )]
        pub device_types: ::std::vec::Vec<DeviceTypeInfo>,
        #[serde(
            rename = "grandChildType",
            default,
            skip_serializing_if = "::std::option::Option::is_none"
        )]
        pub grand_child_type: ::std::option::Option<i32>,
        #[serde(
            rename = "isFree",
            default,
            skip_serializing_if = "::std::option::Option::is_none"
        )]
        pub is_free: ::std::option::Option<i32>,
        #[serde(
            rename = "onShelfVersionCode",
            default,
            skip_serializing_if = "::std::option::Option::is_none"
        )]
        pub on_shelf_version_code: ::std::option::Option<i64>,
        #[serde(
            rename = "onShelfVersionId",
            default,
            skip_serializing_if = "::std::option::Option::is_none"
        )]
        pub on_shelf_version_id: ::std::option::Option<::std::string::String>,
        #[serde(
            rename = "onShelfVersionNumber",
            default,
            skip_serializing_if = "::std::option::Option::is_none"
        )]
        pub on_shelf_version_number: ::std::option::Option<::std::string::String>,
        #[serde(
            rename = "parentType",
            default,
            skip_serializing_if = "::std::option::Option::is_none"
        )]
        pub parent_type: ::std::option::Option<i32>,
        #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
        pub price: ::std::option::Option<::std::string::String>,
        #[serde(
            rename = "priceDetail",
            default,
            skip_serializing_if = "::std::option::Option::is_none"
        )]
        pub price_detail: ::std::option::Option<::std::string::String>,
        #[serde(
            rename = "privacyPolicy",
            default,
            skip_serializing_if = "::std::option::Option::is_none"
        )]
        pub privacy_policy: ::std::option::Option<::std::string::String>,
        #[serde(
            rename = "publishCountry",
            default,
            skip_serializing_if = "::std::option::Option::is_none"
        )]
        pub publish_country: ::std::option::Option<::std::string::String>,
        #[serde(
            rename = "releaseState",
            default,
            skip_serializing_if = "::std::option::Option::is_none"
        )]
        pub release_state: ::std::option::Option<i32>,
        #[serde(
            rename = "shareLink",
            default,
            skip_serializing_if = "::std::option::Option::is_none"
        )]
        pub share_link: ::std::option::Option<::std::string::String>,
        #[serde(
            rename = "versionCode",
            default,
            skip_serializing_if = "::std::option::Option::is_none"
        )]
        pub version_code: ::std::option::Option<i64>,
        #[serde(
            rename = "versionId",
            default,
            skip_serializing_if = "::std::option::Option::is_none"
        )]
        pub version_id: ::std::option::Option<::std::string::String>,
        #[serde(
            rename = "versionNumber",
            default,
            skip_serializing_if = "::std::option::Option::is_none"
        )]
        pub version_number: ::std::option::Option<::std::string::String>,
    }

    impl ::std::default::Default for AppInfo {
        fn default() -> Self {
            Self {
                app_net_type: Default::default(),
                child_type: Default::default(),
                content_rate: Default::default(),
                default_lang: Default::default(),
                device_types: Default::default(),
                grand_child_type: Default::default(),
                is_free: Default::default(),
                on_shelf_version_code: Default::default(),
                on_shelf_version_id: Default::default(),
                on_shelf_version_number: Default::default(),
                parent_type: Default::default(),
                price: Default::default(),
                price_detail: Default::default(),
                privacy_policy: Default::default(),
                publish_country: Default::default(),
                release_state: Default::default(),
                share_link: Default::default(),
                version_code: Default::default(),
                version_id: Default::default(),
                version_number: Default::default(),
            }
        }
    }

    ///`AppInfoResponse`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "allOf": [
    ///    {
    ///      "$ref": "#/components/schemas/ResultResponse"
    ///    },
    ///    {
    ///      "type": "object",
    ///      "properties": {
    ///        "appInfo": {
    ///          "$ref": "#/components/schemas/AppInfo"
    ///        },
    ///        "auditInfo": {
    ///          "$ref": "#/components/schemas/AuditInfo"
    ///        },
    ///        "languages": {
    ///          "type": "array",
    ///          "items": {
    ///            "$ref": "#/components/schemas/LanguageInfo"
    ///          }
    ///        },
    ///        "phasedReleaseInfo": {
    ///          "$ref": "#/components/schemas/PhasedReleaseInfo"
    ///        }
    ///      }
    ///    }
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct AppInfoResponse {
        #[serde(
            rename = "appInfo",
            default,
            skip_serializing_if = "::std::option::Option::is_none"
        )]
        pub app_info: ::std::option::Option<AppInfo>,
        #[serde(
            rename = "auditInfo",
            default,
            skip_serializing_if = "::std::option::Option::is_none"
        )]
        pub audit_info: ::std::option::Option<AuditInfo>,
        #[serde(default, skip_serializing_if = "::std::vec::Vec::is_empty")]
        pub languages: ::std::vec::Vec<LanguageInfo>,
        #[serde(
            rename = "phasedReleaseInfo",
            default,
            skip_serializing_if = "::std::option::Option::is_none"
        )]
        pub phased_release_info: ::std::option::Option<PhasedReleaseInfo>,
        pub ret: Result,
    }

    ///`AppInfoUpdate`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "properties": {
    ///    "childType": {
    ///      "type": "integer",
    ///      "format": "int32"
    ///    },
    ///    "defaultLang": {
    ///      "type": "string"
    ///    },
    ///    "grandChildType": {
    ///      "type": "integer",
    ///      "format": "int32"
    ///    },
    ///    "isFree": {
    ///      "type": "integer",
    ///      "format": "int32"
    ///    },
    ///    "parentType": {
    ///      "type": "integer",
    ///      "format": "int32"
    ///    },
    ///    "privacyPolicy": {
    ///      "type": "string"
    ///    },
    ///    "publishCountry": {
    ///      "type": "string"
    ///    }
    ///  },
    ///  "additionalProperties": true
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct AppInfoUpdate {
        #[serde(
            rename = "childType",
            default,
            skip_serializing_if = "::std::option::Option::is_none"
        )]
        pub child_type: ::std::option::Option<i32>,
        #[serde(
            rename = "defaultLang",
            default,
            skip_serializing_if = "::std::option::Option::is_none"
        )]
        pub default_lang: ::std::option::Option<::std::string::String>,
        #[serde(
            rename = "grandChildType",
            default,
            skip_serializing_if = "::std::option::Option::is_none"
        )]
        pub grand_child_type: ::std::option::Option<i32>,
        #[serde(
            rename = "isFree",
            default,
            skip_serializing_if = "::std::option::Option::is_none"
        )]
        pub is_free: ::std::option::Option<i32>,
        #[serde(
            rename = "parentType",
            default,
            skip_serializing_if = "::std::option::Option::is_none"
        )]
        pub parent_type: ::std::option::Option<i32>,
        #[serde(
            rename = "privacyPolicy",
            default,
            skip_serializing_if = "::std::option::Option::is_none"
        )]
        pub privacy_policy: ::std::option::Option<::std::string::String>,
        #[serde(
            rename = "publishCountry",
            default,
            skip_serializing_if = "::std::option::Option::is_none"
        )]
        pub publish_country: ::std::option::Option<::std::string::String>,
    }

    impl ::std::default::Default for AppInfoUpdate {
        fn default() -> Self {
            Self {
                child_type: Default::default(),
                default_lang: Default::default(),
                grand_child_type: Default::default(),
                is_free: Default::default(),
                parent_type: Default::default(),
                privacy_policy: Default::default(),
                publish_country: Default::default(),
            }
        }
    }

    ///`AuditInfo`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "properties": {
    ///    "auditOpinion": {
    ///      "type": "string"
    ///    },
    ///    "copyRightAuditOpinion": {
    ///      "type": "string"
    ///    },
    ///    "copyRightAuditResult": {
    ///      "type": "integer",
    ///      "format": "int32"
    ///    },
    ///    "recordAuditOpinion": {
    ///      "type": "string"
    ///    },
    ///    "recordAuditResult": {
    ///      "type": "integer",
    ///      "format": "int32"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct AuditInfo {
        #[serde(
            rename = "auditOpinion",
            default,
            skip_serializing_if = "::std::option::Option::is_none"
        )]
        pub audit_opinion: ::std::option::Option<::std::string::String>,
        #[serde(
            rename = "copyRightAuditOpinion",
            default,
            skip_serializing_if = "::std::option::Option::is_none"
        )]
        pub copy_right_audit_opinion: ::std::option::Option<::std::string::String>,
        #[serde(
            rename = "copyRightAuditResult",
            default,
            skip_serializing_if = "::std::option::Option::is_none"
        )]
        pub copy_right_audit_result: ::std::option::Option<i32>,
        #[serde(
            rename = "recordAuditOpinion",
            default,
            skip_serializing_if = "::std::option::Option::is_none"
        )]
        pub record_audit_opinion: ::std::option::Option<::std::string::String>,
        #[serde(
            rename = "recordAuditResult",
            default,
            skip_serializing_if = "::std::option::Option::is_none"
        )]
        pub record_audit_result: ::std::option::Option<i32>,
    }

    impl ::std::default::Default for AuditInfo {
        fn default() -> Self {
            Self {
                audit_opinion: Default::default(),
                copy_right_audit_opinion: Default::default(),
                copy_right_audit_result: Default::default(),
                record_audit_opinion: Default::default(),
                record_audit_result: Default::default(),
            }
        }
    }

    ///`CompileStatusResponse`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "allOf": [
    ///    {
    ///      "$ref": "#/components/schemas/ResultResponse"
    ///    },
    ///    {
    ///      "type": "object",
    ///      "properties": {
    ///        "pkgStateList": {
    ///          "type": "array",
    ///          "items": {
    ///            "$ref": "#/components/schemas/PackageState"
    ///          }
    ///        }
    ///      },
    ///      "additionalProperties": true
    ///    }
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct CompileStatusResponse {
        #[serde(
            rename = "pkgStateList",
            default,
            skip_serializing_if = "::std::vec::Vec::is_empty"
        )]
        pub pkg_state_list: ::std::vec::Vec<PackageState>,
        pub ret: Result,
    }

    ///`DeviceMaterial`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "deviceType"
    ///  ],
    ///  "properties": {
    ///    "appIcon": {
    ///      "type": "string"
    ///    },
    ///    "banner": {
    ///      "type": "string"
    ///    },
    ///    "deviceType": {
    ///      "type": "integer",
    ///      "format": "int32"
    ///    },
    ///    "promoGraphics": {
    ///      "oneOf": [
    ///        {
    ///          "type": "string"
    ///        },
    ///        {
    ///          "type": "array",
    ///          "items": {
    ///            "type": "string"
    ///          }
    ///        }
    ///      ]
    ///    },
    ///    "screenShots": {
    ///      "oneOf": [
    ///        {
    ///          "type": "string"
    ///        },
    ///        {
    ///          "type": "array",
    ///          "items": {
    ///            "type": "string"
    ///          }
    ///        }
    ///      ]
    ///    },
    ///    "showType": {
    ///      "type": "integer",
    ///      "format": "int32"
    ///    },
    ///    "videoShowType": {
    ///      "type": "integer",
    ///      "format": "int32"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct DeviceMaterial {
        #[serde(
            rename = "appIcon",
            default,
            skip_serializing_if = "::std::option::Option::is_none"
        )]
        pub app_icon: ::std::option::Option<::std::string::String>,
        #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
        pub banner: ::std::option::Option<::std::string::String>,
        #[serde(rename = "deviceType")]
        pub device_type: i32,
        #[serde(
            rename = "promoGraphics",
            default,
            skip_serializing_if = "::std::option::Option::is_none"
        )]
        pub promo_graphics: ::std::option::Option<DeviceMaterialPromoGraphics>,
        #[serde(
            rename = "screenShots",
            default,
            skip_serializing_if = "::std::option::Option::is_none"
        )]
        pub screen_shots: ::std::option::Option<DeviceMaterialScreenShots>,
        #[serde(
            rename = "showType",
            default,
            skip_serializing_if = "::std::option::Option::is_none"
        )]
        pub show_type: ::std::option::Option<i32>,
        #[serde(
            rename = "videoShowType",
            default,
            skip_serializing_if = "::std::option::Option::is_none"
        )]
        pub video_show_type: ::std::option::Option<i32>,
    }

    ///`DeviceMaterialPromoGraphics`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "oneOf": [
    ///    {
    ///      "type": "string"
    ///    },
    ///    {
    ///      "type": "array",
    ///      "items": {
    ///        "type": "string"
    ///      }
    ///    }
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    #[serde(untagged)]
    pub enum DeviceMaterialPromoGraphics {
        String(::std::string::String),
        Array(::std::vec::Vec<::std::string::String>),
    }

    impl ::std::convert::From<::std::vec::Vec<::std::string::String>> for DeviceMaterialPromoGraphics {
        fn from(value: ::std::vec::Vec<::std::string::String>) -> Self {
            Self::Array(value)
        }
    }

    ///`DeviceMaterialScreenShots`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "oneOf": [
    ///    {
    ///      "type": "string"
    ///    },
    ///    {
    ///      "type": "array",
    ///      "items": {
    ///        "type": "string"
    ///      }
    ///    }
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    #[serde(untagged)]
    pub enum DeviceMaterialScreenShots {
        String(::std::string::String),
        Array(::std::vec::Vec<::std::string::String>),
    }

    impl ::std::convert::From<::std::vec::Vec<::std::string::String>> for DeviceMaterialScreenShots {
        fn from(value: ::std::vec::Vec<::std::string::String>) -> Self {
            Self::Array(value)
        }
    }

    ///`DeviceTypeInfo`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "deviceType"
    ///  ],
    ///  "properties": {
    ///    "appAdapters": {
    ///      "type": "string"
    ///    },
    ///    "deviceType": {
    ///      "type": "integer",
    ///      "format": "int32"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct DeviceTypeInfo {
        #[serde(
            rename = "appAdapters",
            default,
            skip_serializing_if = "::std::option::Option::is_none"
        )]
        pub app_adapters: ::std::option::Option<::std::string::String>,
        #[serde(rename = "deviceType")]
        pub device_type: i32,
    }

    ///`KeyValue`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "properties": {
    ///    "key": {
    ///      "type": "string"
    ///    },
    ///    "value": {
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct KeyValue {
        #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
        pub key: ::std::option::Option<::std::string::String>,
        #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
        pub value: ::std::option::Option<::std::string::String>,
    }

    impl ::std::default::Default for KeyValue {
        fn default() -> Self {
            Self {
                key: Default::default(),
                value: Default::default(),
            }
        }
    }

    ///`LanguageInfo`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "properties": {
    ///    "appDesc": {
    ///      "type": "string"
    ///    },
    ///    "appName": {
    ///      "type": "string"
    ///    },
    ///    "briefInfo": {
    ///      "type": "string"
    ///    },
    ///    "deviceMaterials": {
    ///      "type": "array",
    ///      "items": {
    ///        "$ref": "#/components/schemas/DeviceMaterial"
    ///      }
    ///    },
    ///    "icon": {
    ///      "type": "string"
    ///    },
    ///    "introPic": {
    ///      "type": "string"
    ///    },
    ///    "introVideo": {
    ///      "type": "string"
    ///    },
    ///    "lang": {
    ///      "type": "string"
    ///    },
    ///    "newFeatures": {
    ///      "type": "string"
    ///    },
    ///    "rcmdPic": {
    ///      "type": "string"
    ///    },
    ///    "rcmdVideo": {
    ///      "type": "string"
    ///    },
    ///    "showType": {
    ///      "type": "integer",
    ///      "format": "int32"
    ///    },
    ///    "videoShowType": {
    ///      "type": "integer",
    ///      "format": "int32"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct LanguageInfo {
        #[serde(
            rename = "appDesc",
            default,
            skip_serializing_if = "::std::option::Option::is_none"
        )]
        pub app_desc: ::std::option::Option<::std::string::String>,
        #[serde(
            rename = "appName",
            default,
            skip_serializing_if = "::std::option::Option::is_none"
        )]
        pub app_name: ::std::option::Option<::std::string::String>,
        #[serde(
            rename = "briefInfo",
            default,
            skip_serializing_if = "::std::option::Option::is_none"
        )]
        pub brief_info: ::std::option::Option<::std::string::String>,
        #[serde(
            rename = "deviceMaterials",
            default,
            skip_serializing_if = "::std::vec::Vec::is_empty"
        )]
        pub device_materials: ::std::vec::Vec<DeviceMaterial>,
        #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
        pub icon: ::std::option::Option<::std::string::String>,
        #[serde(
            rename = "introPic",
            default,
            skip_serializing_if = "::std::option::Option::is_none"
        )]
        pub intro_pic: ::std::option::Option<::std::string::String>,
        #[serde(
            rename = "introVideo",
            default,
            skip_serializing_if = "::std::option::Option::is_none"
        )]
        pub intro_video: ::std::option::Option<::std::string::String>,
        #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
        pub lang: ::std::option::Option<::std::string::String>,
        #[serde(
            rename = "newFeatures",
            default,
            skip_serializing_if = "::std::option::Option::is_none"
        )]
        pub new_features: ::std::option::Option<::std::string::String>,
        #[serde(
            rename = "rcmdPic",
            default,
            skip_serializing_if = "::std::option::Option::is_none"
        )]
        pub rcmd_pic: ::std::option::Option<::std::string::String>,
        #[serde(
            rename = "rcmdVideo",
            default,
            skip_serializing_if = "::std::option::Option::is_none"
        )]
        pub rcmd_video: ::std::option::Option<::std::string::String>,
        #[serde(
            rename = "showType",
            default,
            skip_serializing_if = "::std::option::Option::is_none"
        )]
        pub show_type: ::std::option::Option<i32>,
        #[serde(
            rename = "videoShowType",
            default,
            skip_serializing_if = "::std::option::Option::is_none"
        )]
        pub video_show_type: ::std::option::Option<i32>,
    }

    impl ::std::default::Default for LanguageInfo {
        fn default() -> Self {
            Self {
                app_desc: Default::default(),
                app_name: Default::default(),
                brief_info: Default::default(),
                device_materials: Default::default(),
                icon: Default::default(),
                intro_pic: Default::default(),
                intro_video: Default::default(),
                lang: Default::default(),
                new_features: Default::default(),
                rcmd_pic: Default::default(),
                rcmd_video: Default::default(),
                show_type: Default::default(),
                video_show_type: Default::default(),
            }
        }
    }

    ///`LanguageInfoUpdate`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "lang"
    ///  ],
    ///  "properties": {
    ///    "appDesc": {
    ///      "type": "string"
    ///    },
    ///    "appName": {
    ///      "type": "string"
    ///    },
    ///    "briefInfo": {
    ///      "type": "string"
    ///    },
    ///    "icon": {
    ///      "type": "string"
    ///    },
    ///    "introPic": {
    ///      "type": "string"
    ///    },
    ///    "lang": {
    ///      "type": "string"
    ///    },
    ///    "newFeatures": {
    ///      "type": "string"
    ///    }
    ///  },
    ///  "additionalProperties": true
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct LanguageInfoUpdate {
        #[serde(
            rename = "appDesc",
            default,
            skip_serializing_if = "::std::option::Option::is_none"
        )]
        pub app_desc: ::std::option::Option<::std::string::String>,
        #[serde(
            rename = "appName",
            default,
            skip_serializing_if = "::std::option::Option::is_none"
        )]
        pub app_name: ::std::option::Option<::std::string::String>,
        #[serde(
            rename = "briefInfo",
            default,
            skip_serializing_if = "::std::option::Option::is_none"
        )]
        pub brief_info: ::std::option::Option<::std::string::String>,
        #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
        pub icon: ::std::option::Option<::std::string::String>,
        #[serde(
            rename = "introPic",
            default,
            skip_serializing_if = "::std::option::Option::is_none"
        )]
        pub intro_pic: ::std::option::Option<::std::string::String>,
        pub lang: ::std::string::String,
        #[serde(
            rename = "newFeatures",
            default,
            skip_serializing_if = "::std::option::Option::is_none"
        )]
        pub new_features: ::std::option::Option<::std::string::String>,
    }

    ///`PackageInfo`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "fileName",
    ///    "packageSize",
    ///    "packageType",
    ///    "pkgVersion",
    ///    "uploadTime",
    ///    "versionCode"
    ///  ],
    ///  "properties": {
    ///    "fileName": {
    ///      "type": "string"
    ///    },
    ///    "packageSize": {
    ///      "type": "integer",
    ///      "format": "int64"
    ///    },
    ///    "packageType": {
    ///      "type": "integer",
    ///      "format": "int32"
    ///    },
    ///    "pkgVersion": {
    ///      "type": "string"
    ///    },
    ///    "shaCode": {
    ///      "type": "string"
    ///    },
    ///    "uploadTime": {
    ///      "type": "integer",
    ///      "format": "int64"
    ///    },
    ///    "versionCode": {
    ///      "type": "integer",
    ///      "format": "int64"
    ///    },
    ///    "versionName": {
    ///      "type": "string"
    ///    }
    ///  },
    ///  "additionalProperties": true
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct PackageInfo {
        #[serde(rename = "fileName")]
        pub file_name: ::std::string::String,
        #[serde(rename = "packageSize")]
        pub package_size: i64,
        #[serde(rename = "packageType")]
        pub package_type: i32,
        #[serde(rename = "pkgVersion")]
        pub pkg_version: ::std::string::String,
        #[serde(
            rename = "shaCode",
            default,
            skip_serializing_if = "::std::option::Option::is_none"
        )]
        pub sha_code: ::std::option::Option<::std::string::String>,
        #[serde(rename = "uploadTime")]
        pub upload_time: i64,
        #[serde(rename = "versionCode")]
        pub version_code: i64,
        #[serde(
            rename = "versionName",
            default,
            skip_serializing_if = "::std::option::Option::is_none"
        )]
        pub version_name: ::std::option::Option<::std::string::String>,
    }

    ///`PackageListResponse`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "allOf": [
    ///    {
    ///      "$ref": "#/components/schemas/ResultResponse"
    ///    },
    ///    {
    ///      "type": "object",
    ///      "properties": {
    ///        "abisType": {
    ///          "type": "integer",
    ///          "format": "int32"
    ///        },
    ///        "packagePath": {
    ///          "type": "string"
    ///        },
    ///        "pkgList": {
    ///          "type": "array",
    ///          "items": {
    ///            "$ref": "#/components/schemas/PackageInfo"
    ///          }
    ///        },
    ///        "retCount": {
    ///          "type": "integer",
    ///          "format": "int32"
    ///        },
    ///        "totalCount": {
    ///          "type": "integer",
    ///          "format": "int32"
    ///        }
    ///      }
    ///    }
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct PackageListResponse {
        #[serde(
            rename = "abisType",
            default,
            skip_serializing_if = "::std::option::Option::is_none"
        )]
        pub abis_type: ::std::option::Option<i32>,
        #[serde(
            rename = "packagePath",
            default,
            skip_serializing_if = "::std::option::Option::is_none"
        )]
        pub package_path: ::std::option::Option<::std::string::String>,
        #[serde(
            rename = "pkgList",
            default,
            skip_serializing_if = "::std::vec::Vec::is_empty"
        )]
        pub pkg_list: ::std::vec::Vec<PackageInfo>,
        pub ret: Result,
        #[serde(
            rename = "retCount",
            default,
            skip_serializing_if = "::std::option::Option::is_none"
        )]
        pub ret_count: ::std::option::Option<i32>,
        #[serde(
            rename = "totalCount",
            default,
            skip_serializing_if = "::std::option::Option::is_none"
        )]
        pub total_count: ::std::option::Option<i32>,
    }

    ///`PackageState`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "pkgId",
    ///    "successStatus"
    ///  ],
    ///  "properties": {
    ///    "aabCompileStatus": {
    ///      "deprecated": true,
    ///      "type": "integer",
    ///      "format": "int32"
    ///    },
    ///    "failReason": {
    ///      "deprecated": true,
    ///      "type": "integer",
    ///      "format": "int32"
    ///    },
    ///    "pkgId": {
    ///      "type": "string"
    ///    },
    ///    "successStatus": {
    ///      "type": "integer",
    ///      "format": "int32"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct PackageState {
        #[serde(
            rename = "aabCompileStatus",
            default,
            skip_serializing_if = "::std::option::Option::is_none"
        )]
        pub aab_compile_status: ::std::option::Option<i32>,
        #[serde(
            rename = "failReason",
            default,
            skip_serializing_if = "::std::option::Option::is_none"
        )]
        pub fail_reason: ::std::option::Option<i32>,
        #[serde(rename = "pkgId")]
        pub pkg_id: ::std::string::String,
        #[serde(rename = "successStatus")]
        pub success_status: i32,
    }

    ///`PhasedReleaseInfo`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "properties": {
    ///    "phasedReleaseDescription": {
    ///      "type": "string"
    ///    },
    ///    "phasedReleaseEndTime": {
    ///      "type": "string"
    ///    },
    ///    "phasedReleasePercent": {
    ///      "type": "string"
    ///    },
    ///    "phasedReleaseStartTime": {
    ///      "type": "string"
    ///    },
    ///    "state": {
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct PhasedReleaseInfo {
        #[serde(
            rename = "phasedReleaseDescription",
            default,
            skip_serializing_if = "::std::option::Option::is_none"
        )]
        pub phased_release_description: ::std::option::Option<::std::string::String>,
        #[serde(
            rename = "phasedReleaseEndTime",
            default,
            skip_serializing_if = "::std::option::Option::is_none"
        )]
        pub phased_release_end_time: ::std::option::Option<::std::string::String>,
        #[serde(
            rename = "phasedReleasePercent",
            default,
            skip_serializing_if = "::std::option::Option::is_none"
        )]
        pub phased_release_percent: ::std::option::Option<::std::string::String>,
        #[serde(
            rename = "phasedReleaseStartTime",
            default,
            skip_serializing_if = "::std::option::Option::is_none"
        )]
        pub phased_release_start_time: ::std::option::Option<::std::string::String>,
        #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
        pub state: ::std::option::Option<::std::string::String>,
    }

    impl ::std::default::Default for PhasedReleaseInfo {
        fn default() -> Self {
            Self {
                phased_release_description: Default::default(),
                phased_release_end_time: Default::default(),
                phased_release_percent: Default::default(),
                phased_release_start_time: Default::default(),
                state: Default::default(),
            }
        }
    }

    ///`Result`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "code",
    ///    "msg"
    ///  ],
    ///  "properties": {
    ///    "code": {
    ///      "type": "integer",
    ///      "format": "int64"
    ///    },
    ///    "msg": {
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct Result {
        pub code: i64,
        pub msg: ::std::string::String,
    }

    ///`ResultResponse`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "ret"
    ///  ],
    ///  "properties": {
    ///    "ret": {
    ///      "$ref": "#/components/schemas/Result"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct ResultResponse {
        pub ret: Result,
    }

    ///`UploadHeader`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "name",
    ///    "value"
    ///  ],
    ///  "properties": {
    ///    "name": {
    ///      "type": "string"
    ///    },
    ///    "value": {
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct UploadHeader {
        pub name: ::std::string::String,
        pub value: ::std::string::String,
    }

    ///`UploadUrlInfo`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "headers",
    ///    "objectId",
    ///    "url"
    ///  ],
    ///  "properties": {
    ///    "headers": {
    ///      "type": "array",
    ///      "items": {
    ///        "$ref": "#/components/schemas/UploadHeader"
    ///      }
    ///    },
    ///    "objectId": {
    ///      "type": "string"
    ///    },
    ///    "url": {
    ///      "type": "string",
    ///      "format": "uri"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct UploadUrlInfo {
        pub headers: ::std::vec::Vec<UploadHeader>,
        #[serde(rename = "objectId")]
        pub object_id: ::std::string::String,
        pub url: ::std::string::String,
    }

    ///`UploadUrlResponse`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "allOf": [
    ///    {
    ///      "$ref": "#/components/schemas/ResultResponse"
    ///    },
    ///    {
    ///      "type": "object",
    ///      "properties": {
    ///        "urlInfo": {
    ///          "$ref": "#/components/schemas/UploadUrlInfo"
    ///        }
    ///      }
    ///    }
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct UploadUrlResponse {
        pub ret: Result,
        #[serde(
            rename = "urlInfo",
            default,
            skip_serializing_if = "::std::option::Option::is_none"
        )]
        pub url_info: ::std::option::Option<UploadUrlInfo>,
    }

    ///`UploadedPackage`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "fileName",
    ///    "objectId"
    ///  ],
    ///  "properties": {
    ///    "fileName": {
    ///      "type": "string"
    ///    },
    ///    "objectId": {
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct UploadedPackage {
        #[serde(rename = "fileName")]
        pub file_name: ::std::string::String,
        #[serde(rename = "objectId")]
        pub object_id: ::std::string::String,
    }
}

#[derive(Clone, Debug)]
///Client for AppGallery Connect Publishing API
///
///OpenAPI description normalized from the official Huawei AppGallery Connect
/// Publishing API reference. Huawei publishes the endpoints as HTML reference
/// pages rather than as a downloadable OpenAPI document.
///
///Version: 2025-09-30
pub struct Client {
    pub(crate) baseurl: String,
    pub(crate) client: reqwest::Client,
}

impl Client {
    /// Create a new client.
    ///
    /// `baseurl` is the base URL provided to the internal
    /// `reqwest::Client`, and should include a scheme and hostname,
    /// as well as port and a path stem if applicable.
    pub fn new(baseurl: &str) -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        let client = {
            let dur = ::std::time::Duration::from_secs(15u64);
            reqwest::ClientBuilder::new()
                .connect_timeout(dur)
                .timeout(dur)
        };
        #[cfg(target_arch = "wasm32")]
        let client = reqwest::ClientBuilder::new();
        Self::new_with_client(baseurl, client.build().unwrap())
    }

    /// Construct a new client with an existing `reqwest::Client`,
    /// allowing more control over its configuration.
    ///
    /// `baseurl` is the base URL provided to the internal
    /// `reqwest::Client`, and should include a scheme and hostname,
    /// as well as port and a path stem if applicable.
    pub fn new_with_client(baseurl: &str, client: reqwest::Client) -> Self {
        Self {
            baseurl: baseurl.to_string(),
            client,
        }
    }
}

impl ClientInfo<()> for Client {
    fn api_version() -> &'static str {
        "2025-09-30"
    }

    fn baseurl(&self) -> &str {
        self.baseurl.as_str()
    }

    fn client(&self) -> &reqwest::Client {
        &self.client
    }

    fn inner(&self) -> &() {
        &()
    }
}

impl ClientHooks<()> for &Client {}
#[allow(clippy::all)]
impl Client {
    ///Query app IDs by package name
    ///
    ///Sends a `GET` request to `/api/publish/v2/appid-list`
    ///
    ///Arguments:
    /// - `package_name`
    /// - `package_types`
    /// - `pc_version_name`
    /// - `client_id`: Required for legacy API client authentication; omit for
    ///   service accounts.
    pub async fn get_app_ids<'a>(
        &'a self,
        package_name: &'a str,
        package_types: Option<&'a str>,
        pc_version_name: Option<&'a str>,
        client_id: Option<&'a str>,
    ) -> Result<ResponseValue<types::AppIdsResponse>, Error<()>> {
        let url = format!("{}/api/publish/v2/appid-list", self.baseurl,);
        let mut header_map = ::reqwest::header::HeaderMap::with_capacity(2usize);
        header_map.append(
            ::reqwest::header::HeaderName::from_static("api-version"),
            ::reqwest::header::HeaderValue::from_static(Self::api_version()),
        );
        if let Some(value) = client_id {
            header_map.append("client_id", value.to_string().try_into()?);
        }

        #[allow(unused_mut)]
        let mut request = self
            .client
            .get(url)
            .header(
                ::reqwest::header::ACCEPT,
                ::reqwest::header::HeaderValue::from_static("application/json"),
            )
            .query(&progenitor_client::QueryParam::new(
                "packageName",
                &package_name,
            ))
            .query(&progenitor_client::QueryParam::new(
                "packageTypes",
                &package_types,
            ))
            .query(&progenitor_client::QueryParam::new(
                "pcVersionName",
                &pc_version_name,
            ))
            .headers(header_map)
            .build()?;
        let info = OperationInfo {
            operation_id: "get_app_ids",
        };
        self.pre(&mut request, &info).await?;
        let result = self.exec(request, &info).await;
        self.post(&result, &info).await?;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }

    ///Query app information
    ///
    ///Sends a `GET` request to `/api/publish/v2/app-info`
    ///
    ///Arguments:
    /// - `app_id`
    /// - `lang`
    /// - `release_type`
    /// - `client_id`: Required for legacy API client authentication; omit for
    ///   service accounts.
    pub async fn get_app_info<'a>(
        &'a self,
        app_id: &'a str,
        lang: Option<&'a str>,
        release_type: Option<i32>,
        client_id: Option<&'a str>,
    ) -> Result<ResponseValue<types::AppInfoResponse>, Error<()>> {
        let url = format!("{}/api/publish/v2/app-info", self.baseurl,);
        let mut header_map = ::reqwest::header::HeaderMap::with_capacity(2usize);
        header_map.append(
            ::reqwest::header::HeaderName::from_static("api-version"),
            ::reqwest::header::HeaderValue::from_static(Self::api_version()),
        );
        if let Some(value) = client_id {
            header_map.append("client_id", value.to_string().try_into()?);
        }

        #[allow(unused_mut)]
        let mut request = self
            .client
            .get(url)
            .header(
                ::reqwest::header::ACCEPT,
                ::reqwest::header::HeaderValue::from_static("application/json"),
            )
            .query(&progenitor_client::QueryParam::new("appId", &app_id))
            .query(&progenitor_client::QueryParam::new("lang", &lang))
            .query(&progenitor_client::QueryParam::new(
                "releaseType",
                &release_type,
            ))
            .headers(header_map)
            .build()?;
        let info = OperationInfo {
            operation_id: "get_app_info",
        };
        self.pre(&mut request, &info).await?;
        let result = self.exec(request, &info).await;
        self.post(&result, &info).await?;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }

    ///Update app basic information
    ///
    ///Sends a `PUT` request to `/api/publish/v2/app-info`
    ///
    ///Arguments:
    /// - `app_id`
    /// - `release_type`
    /// - `client_id`: Required for legacy API client authentication; omit for
    ///   service accounts.
    /// - `body`
    pub async fn update_app_info<'a>(
        &'a self,
        app_id: &'a str,
        release_type: Option<i32>,
        client_id: Option<&'a str>,
        body: &'a types::AppInfoUpdate,
    ) -> Result<ResponseValue<types::ResultResponse>, Error<()>> {
        let url = format!("{}/api/publish/v2/app-info", self.baseurl,);
        let mut header_map = ::reqwest::header::HeaderMap::with_capacity(2usize);
        header_map.append(
            ::reqwest::header::HeaderName::from_static("api-version"),
            ::reqwest::header::HeaderValue::from_static(Self::api_version()),
        );
        if let Some(value) = client_id {
            header_map.append("client_id", value.to_string().try_into()?);
        }

        #[allow(unused_mut)]
        let mut request = self
            .client
            .put(url)
            .header(
                ::reqwest::header::ACCEPT,
                ::reqwest::header::HeaderValue::from_static("application/json"),
            )
            .json(&body)
            .query(&progenitor_client::QueryParam::new("appId", &app_id))
            .query(&progenitor_client::QueryParam::new(
                "releaseType",
                &release_type,
            ))
            .headers(header_map)
            .build()?;
        let info = OperationInfo {
            operation_id: "update_app_info",
        };
        self.pre(&mut request, &info).await?;
        let result = self.exec(request, &info).await;
        self.post(&result, &info).await?;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }

    ///Create or update app localization information
    ///
    ///Sends a `PUT` request to `/api/publish/v2/app-language-info`
    ///
    ///Arguments:
    /// - `app_id`
    /// - `release_type`
    /// - `client_id`: Required for legacy API client authentication; omit for
    ///   service accounts.
    /// - `body`
    pub async fn update_app_language_info<'a>(
        &'a self,
        app_id: &'a str,
        release_type: Option<i32>,
        client_id: Option<&'a str>,
        body: &'a types::LanguageInfoUpdate,
    ) -> Result<ResponseValue<types::ResultResponse>, Error<()>> {
        let url = format!("{}/api/publish/v2/app-language-info", self.baseurl,);
        let mut header_map = ::reqwest::header::HeaderMap::with_capacity(2usize);
        header_map.append(
            ::reqwest::header::HeaderName::from_static("api-version"),
            ::reqwest::header::HeaderValue::from_static(Self::api_version()),
        );
        if let Some(value) = client_id {
            header_map.append("client_id", value.to_string().try_into()?);
        }

        #[allow(unused_mut)]
        let mut request = self
            .client
            .put(url)
            .header(
                ::reqwest::header::ACCEPT,
                ::reqwest::header::HeaderValue::from_static("application/json"),
            )
            .json(&body)
            .query(&progenitor_client::QueryParam::new("appId", &app_id))
            .query(&progenitor_client::QueryParam::new(
                "releaseType",
                &release_type,
            ))
            .headers(header_map)
            .build()?;
        let info = OperationInfo {
            operation_id: "update_app_language_info",
        };
        self.pre(&mut request, &info).await?;
        let result = self.exec(request, &info).await;
        self.post(&result, &info).await?;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }

    ///Delete app localization information
    ///
    ///Sends a `DELETE` request to `/api/publish/v2/app-language-info`
    ///
    ///Arguments:
    /// - `app_id`
    /// - `lang`
    /// - `release_type`
    /// - `client_id`: Required for legacy API client authentication; omit for
    ///   service accounts.
    pub async fn delete_app_language_info<'a>(
        &'a self,
        app_id: &'a str,
        lang: &'a str,
        release_type: Option<i32>,
        client_id: Option<&'a str>,
    ) -> Result<ResponseValue<types::ResultResponse>, Error<()>> {
        let url = format!("{}/api/publish/v2/app-language-info", self.baseurl,);
        let mut header_map = ::reqwest::header::HeaderMap::with_capacity(2usize);
        header_map.append(
            ::reqwest::header::HeaderName::from_static("api-version"),
            ::reqwest::header::HeaderValue::from_static(Self::api_version()),
        );
        if let Some(value) = client_id {
            header_map.append("client_id", value.to_string().try_into()?);
        }

        #[allow(unused_mut)]
        let mut request = self
            .client
            .delete(url)
            .header(
                ::reqwest::header::ACCEPT,
                ::reqwest::header::HeaderValue::from_static("application/json"),
            )
            .query(&progenitor_client::QueryParam::new("appId", &app_id))
            .query(&progenitor_client::QueryParam::new("lang", &lang))
            .query(&progenitor_client::QueryParam::new(
                "releaseType",
                &release_type,
            ))
            .headers(header_map)
            .build()?;
        let info = OperationInfo {
            operation_id: "delete_app_language_info",
        };
        self.pre(&mut request, &info).await?;
        let result = self.exec(request, &info).await;
        self.post(&result, &info).await?;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }

    ///Query app package list
    ///
    ///Sends a `GET` request to `/api/publish/v2/package-list`
    ///
    ///Arguments:
    /// - `app_id`
    /// - `from_rec_count`
    /// - `max_req_count`
    /// - `client_id`: Required for legacy API client authentication; omit for
    ///   service accounts.
    pub async fn get_package_list<'a>(
        &'a self,
        app_id: &'a str,
        from_rec_count: Option<i32>,
        max_req_count: Option<i32>,
        client_id: Option<&'a str>,
    ) -> Result<ResponseValue<types::PackageListResponse>, Error<()>> {
        let url = format!("{}/api/publish/v2/package-list", self.baseurl,);
        let mut header_map = ::reqwest::header::HeaderMap::with_capacity(2usize);
        header_map.append(
            ::reqwest::header::HeaderName::from_static("api-version"),
            ::reqwest::header::HeaderValue::from_static(Self::api_version()),
        );
        if let Some(value) = client_id {
            header_map.append("client_id", value.to_string().try_into()?);
        }

        #[allow(unused_mut)]
        let mut request = self
            .client
            .get(url)
            .header(
                ::reqwest::header::ACCEPT,
                ::reqwest::header::HeaderValue::from_static("application/json"),
            )
            .query(&progenitor_client::QueryParam::new("appId", &app_id))
            .query(&progenitor_client::QueryParam::new(
                "fromRecCount",
                &from_rec_count,
            ))
            .query(&progenitor_client::QueryParam::new(
                "maxReqCount",
                &max_req_count,
            ))
            .headers(header_map)
            .build()?;
        let info = OperationInfo {
            operation_id: "get_package_list",
        };
        self.pre(&mut request, &info).await?;
        let result = self.exec(request, &info).await;
        self.post(&result, &info).await?;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }

    ///Query AAB compilation status
    ///
    ///Sends a `GET` request to `/api/publish/v2/package/compile/status`
    ///
    ///Arguments:
    /// - `app_id`
    /// - `pkg_ids`
    /// - `client_id`: Required for legacy API client authentication; omit for
    ///   service accounts.
    pub async fn get_package_compile_status<'a>(
        &'a self,
        app_id: &'a str,
        pkg_ids: &'a str,
        client_id: Option<&'a str>,
    ) -> Result<ResponseValue<types::CompileStatusResponse>, Error<()>> {
        let url = format!("{}/api/publish/v2/package/compile/status", self.baseurl,);
        let mut header_map = ::reqwest::header::HeaderMap::with_capacity(2usize);
        header_map.append(
            ::reqwest::header::HeaderName::from_static("api-version"),
            ::reqwest::header::HeaderValue::from_static(Self::api_version()),
        );
        if let Some(value) = client_id {
            header_map.append("client_id", value.to_string().try_into()?);
        }

        #[allow(unused_mut)]
        let mut request = self
            .client
            .get(url)
            .header(
                ::reqwest::header::ACCEPT,
                ::reqwest::header::HeaderValue::from_static("application/json"),
            )
            .query(&progenitor_client::QueryParam::new("appId", &app_id))
            .query(&progenitor_client::QueryParam::new("pkgIds", &pkg_ids))
            .headers(header_map)
            .build()?;
        let info = OperationInfo {
            operation_id: "get_package_compile_status",
        };
        self.pre(&mut request, &info).await?;
        let result = self.exec(request, &info).await;
        self.post(&result, &info).await?;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }

    ///Obtain a file upload URL
    ///
    ///Sends a `GET` request to `/api/publish/v2/upload-url/for-obs`
    ///
    ///Arguments:
    /// - `app_id`
    /// - `content_length`
    /// - `file_name`
    /// - `client_id`: Required for legacy API client authentication; omit for
    ///   service accounts.
    pub async fn get_upload_url<'a>(
        &'a self,
        app_id: &'a str,
        content_length: i64,
        file_name: &'a str,
        client_id: Option<&'a str>,
    ) -> Result<ResponseValue<types::UploadUrlResponse>, Error<()>> {
        let url = format!("{}/api/publish/v2/upload-url/for-obs", self.baseurl,);
        let mut header_map = ::reqwest::header::HeaderMap::with_capacity(2usize);
        header_map.append(
            ::reqwest::header::HeaderName::from_static("api-version"),
            ::reqwest::header::HeaderValue::from_static(Self::api_version()),
        );
        if let Some(value) = client_id {
            header_map.append("client_id", value.to_string().try_into()?);
        }

        #[allow(unused_mut)]
        let mut request = self
            .client
            .get(url)
            .header(
                ::reqwest::header::ACCEPT,
                ::reqwest::header::HeaderValue::from_static("application/json"),
            )
            .query(&progenitor_client::QueryParam::new("appId", &app_id))
            .query(&progenitor_client::QueryParam::new(
                "contentLength",
                &content_length,
            ))
            .query(&progenitor_client::QueryParam::new("fileName", &file_name))
            .headers(header_map)
            .build()?;
        let info = OperationInfo {
            operation_id: "get_upload_url",
        };
        self.pre(&mut request, &info).await?;
        let result = self.exec(request, &info).await;
        self.post(&result, &info).await?;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }

    ///Submit uploaded package information
    ///
    ///Sends a `PUT` request to `/api/publish/v3/app-package-info`
    ///
    ///Arguments:
    /// - `app_id`
    /// - `release_phase`
    /// - `release_type`
    /// - `client_id`: Required for legacy API client authentication; omit for
    ///   service accounts.
    /// - `body`
    pub async fn apply_uploaded_package<'a>(
        &'a self,
        app_id: &'a str,
        release_phase: Option<i32>,
        release_type: Option<i32>,
        client_id: Option<&'a str>,
        body: &'a types::UploadedPackage,
    ) -> Result<ResponseValue<types::ResultResponse>, Error<()>> {
        let url = format!("{}/api/publish/v3/app-package-info", self.baseurl,);
        let mut header_map = ::reqwest::header::HeaderMap::with_capacity(2usize);
        header_map.append(
            ::reqwest::header::HeaderName::from_static("api-version"),
            ::reqwest::header::HeaderValue::from_static(Self::api_version()),
        );
        if let Some(value) = client_id {
            header_map.append("client_id", value.to_string().try_into()?);
        }

        #[allow(unused_mut)]
        let mut request = self
            .client
            .put(url)
            .header(
                ::reqwest::header::ACCEPT,
                ::reqwest::header::HeaderValue::from_static("application/json"),
            )
            .json(&body)
            .query(&progenitor_client::QueryParam::new("appId", &app_id))
            .query(&progenitor_client::QueryParam::new(
                "releasePhase",
                &release_phase,
            ))
            .query(&progenitor_client::QueryParam::new(
                "releaseType",
                &release_type,
            ))
            .headers(header_map)
            .build()?;
        let info = OperationInfo {
            operation_id: "apply_uploaded_package",
        };
        self.pre(&mut request, &info).await?;
        let result = self.exec(request, &info).await;
        self.post(&result, &info).await?;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }

    ///Submit an app for review
    ///
    ///Sends a `POST` request to `/api/publish/v2/app-submit`
    ///
    ///Arguments:
    /// - `app_id`
    /// - `release_time`
    /// - `release_type`
    /// - `client_id`: Required for legacy API client authentication; omit for
    ///   service accounts.
    pub async fn submit_app<'a>(
        &'a self,
        app_id: &'a str,
        release_time: Option<&'a str>,
        release_type: Option<i32>,
        client_id: Option<&'a str>,
    ) -> Result<ResponseValue<types::ResultResponse>, Error<()>> {
        let url = format!("{}/api/publish/v2/app-submit", self.baseurl,);
        let mut header_map = ::reqwest::header::HeaderMap::with_capacity(2usize);
        header_map.append(
            ::reqwest::header::HeaderName::from_static("api-version"),
            ::reqwest::header::HeaderValue::from_static(Self::api_version()),
        );
        if let Some(value) = client_id {
            header_map.append("client_id", value.to_string().try_into()?);
        }

        #[allow(unused_mut)]
        let mut request = self
            .client
            .post(url)
            .header(
                ::reqwest::header::ACCEPT,
                ::reqwest::header::HeaderValue::from_static("application/json"),
            )
            .query(&progenitor_client::QueryParam::new("appId", &app_id))
            .query(&progenitor_client::QueryParam::new(
                "releaseTime",
                &release_time,
            ))
            .query(&progenitor_client::QueryParam::new(
                "releaseType",
                &release_type,
            ))
            .headers(header_map)
            .build()?;
        let info = OperationInfo {
            operation_id: "submit_app",
        };
        self.pre(&mut request, &info).await?;
        let result = self.exec(request, &info).await;
        self.post(&result, &info).await?;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
}

/// Items consumers will typically use such as the Client.
pub mod prelude {
    #[allow(unused_imports)]
    pub use super::Client;
}
