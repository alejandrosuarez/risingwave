// Copyright 2023 RisingWave Labs
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use opendal::layers::{LoggingLayer, RetryLayer};
use opendal::services::Oss;
use opendal::Operator;

use super::{EngineType, OpendalObjectStore};
use crate::object::ObjectResult;

/// The minimum number of bytes that is buffered before they are uploaded as a part, , will be used
/// in streaing upload.
///
/// Reference: <https://www.alibabacloud.com/help/en/oss/user-guide/multipart-upload-12>
const OSS_PART_SIZE: usize = 16 * 1024 * 1024;

impl OpendalObjectStore {
    /// create opendal oss engine.
    pub fn new_oss_engine(bucket: String, root: String) -> ObjectResult<Self> {
        // Create oss backend builder.
        let mut builder = Oss::default();

        builder.bucket(&bucket);

        builder.write_min_size(OSS_PART_SIZE);

        builder.root(&root);

        let endpoint = std::env::var("OSS_ENDPOINT")
            .unwrap_or_else(|_| panic!("OSS_ENDPOINT not found from environment variables"));
        let access_key_id = std::env::var("OSS_ACCESS_KEY_ID")
            .unwrap_or_else(|_| panic!("OSS_ACCESS_KEY_ID not found from environment variables"));
        let access_key_secret = std::env::var("OSS_ACCESS_KEY_SECRET").unwrap_or_else(|_| {
            panic!("OSS_ACCESS_KEY_SECRET not found from environment variables")
        });

        builder.endpoint(&endpoint);
        builder.access_key_id(&access_key_id);
        builder.access_key_secret(&access_key_secret);
        let op: Operator = Operator::new(builder)?
            .layer(LoggingLayer::default())
            .layer(RetryLayer::default())
            .finish();
        Ok(Self {
            op,
            engine_type: EngineType::Oss,
        })
    }
}