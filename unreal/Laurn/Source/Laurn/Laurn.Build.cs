// Copyright 2026 Darwin Clay O. and Lawrence Obina
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

using UnrealBuildTool;
using System.IO;

public class Laurn : ModuleRules
{
	public Laurn(ReadOnlyTargetRules Target) : base(Target)
	{
		PCHUsage = ModuleRules.PCHUsageMode.UseExplicitOrSharedPCHs;
		
		PublicIncludePaths.AddRange(
			new string[] {
				// ... add public include paths required here ...
			}
			);
				
		
		PrivateIncludePaths.AddRange(
			new string[] {
				// ... add other private include paths required here ...
			}
			);
			
		
		PublicDependencyModuleNames.AddRange(
			new string[]
			{
				"Core",
				// ... add other public dependencies that you statically link with here ...
			}
			);
			
		
		PrivateDependencyModuleNames.AddRange(
			new string[]
			{
				"CoreUObject",
				"Engine",
				"Slate",
				"SlateCore",
				// ... add private dependencies that you statically link with here ...	
			}
			);
		
		
		DynamicallyLoadedModuleNames.AddRange(
			new string[]
			{
				// ... add any modules that your module loads dynamically here ...
			}
			);
            
        // Configure static linking of the Rust FFI library
        string PluginPath = Utils.MakePathRelativeTo(ModuleDirectory, Target.RelativeEnginePath);
        string RustLibDir = Path.Combine(PluginPath, "../../../../target/debug");
        
        if (Target.Platform == UnrealTargetPlatform.Win64)
        {
            PublicAdditionalLibraries.Add(Path.Combine(RustLibDir, "laurn_c.lib"));
            // Windows typically needs these for standard library linking
            PublicAdditionalLibraries.Add("advapi32.lib");
            PublicAdditionalLibraries.Add("ws2_32.lib");
            PublicAdditionalLibraries.Add("userenv.lib");
            PublicAdditionalLibraries.Add("bcrypt.lib");
        }
        else if (Target.Platform == UnrealTargetPlatform.Mac)
        {
            PublicAdditionalLibraries.Add(Path.Combine(RustLibDir, "liblaurn_c.a"));
            // Mac typically needs iconv and System framework for rust standard lib
            PublicAdditionalLibraries.Add("iconv");
            PublicFrameworks.Add("System");
            PublicFrameworks.Add("CoreFoundation");
        }
        else if (Target.Platform == UnrealTargetPlatform.Linux)
        {
            PublicAdditionalLibraries.Add(Path.Combine(RustLibDir, "liblaurn_c.a"));
        }
    }
}
