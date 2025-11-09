# Rust_for_exif
### 调教AI

- C++不会看不懂，最近在学Rust，就先试试看


---
#### 重新使用little_exif(0.6.16)
- 增加了对于png格式的exif读取
  - 主要就是生成`.xmp.xml`，再读取，构建ExifTag信息输出

---
##### png xmp.xml 转换字符为
```rust
XML:com.adobe.xmp<?xpacket begin="" id="W5M0MpCehiHzreSzNTczkc9d"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/" x:xmptk="Adobe XMP Core 5.6-c140 79.160451, 2017/05/06-01:08:21        ">
 <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
  <rdf:Description rdf:about=""
    xmlns:xmp="http://ns.adobe.com/xap/1.0/"
    xmlns:aux="http://ns.adobe.com/exif/1.0/aux/"
    xmlns:photoshop="http://ns.adobe.com/photoshop/1.0/"
    xmlns:xmpMM="http://ns.adobe.com/xap/1.0/mm/"
    xmlns:stEvt="http://ns.adobe.com/xap/1.0/sType/ResourceEvent#"
    xmlns:stRef="http://ns.adobe.com/xap/1.0/sType/ResourceRef#"
    xmlns:dc="http://purl.org/dc/elements/1.1/"
    xmlns:crs="http://ns.adobe.com/camera-raw-settings/1.0/"
    xmlns:tiff="http://ns.adobe.com/tiff/1.0/"
    xmlns:exif="http://ns.adobe.com/exif/1.0/"
   xmp:CreatorTool="Adobe Photoshop Camera Raw 13.4 (Windows)"
   xmp:CreateDate="2025-11-03T21:11:27.63+08:00"
   xmp:ModifyDate="2025-11-03T22:37:19+08:00"
   xmp:MetadataDate="2025-11-03T22:37:19+08:00"
   aux:SerialNumber="3005098"
   aux:LensInfo="700/10 2100/10 40/10 56/10"
   aux:Lens="70.0-210.0 mm f/4.0-5.6"
   aux:LensID="18"
   aux:ImageNumber="27106"
   aux:ApproximateFocusDistance="2/100"
   photoshop:DateCreated="2025-11-03T21:11:27.63+08:00"
   xmpMM:DocumentID="xmp.did:fd2ea780-c31b-1042-8398-17f130a48fba"
   xmpMM:OriginalDocumentID="03FB35FD3EF347088E046579A6CBFEF4"
   xmpMM:InstanceID="xmp.iid:fd2ea780-c31b-1042-8398-17f130a48fba"
   dc:format="image/png"
   crs:RawFileName="DSC_0128.NEF"
   crs:Version="13.3"
   crs:ProcessVersion="11.0"
   crs:WhiteBalance="Custom"
   crs:Temperature="4350"
   crs:Tint="+7"
   crs:Exposure2012="-1.55"
   crs:Contrast2012="+16"
   crs:Highlights2012="-100"
   crs:Shadows2012="+61"
   crs:Whites2012="-24"
   crs:Blacks2012="+72"
   crs:Texture="0"
   crs:Clarity2012="+23"
   crs:Dehaze="+36"
   crs:Vibrance="0"
   crs:Saturation="0"
   crs:ParametricShadows="+21"
   crs:ParametricDarks="0"
   crs:ParametricLights="+20"
   crs:ParametricHighlights="0"
   crs:ParametricShadowSplit="25"
   crs:ParametricMidtoneSplit="50"
   crs:ParametricHighlightSplit="75"
   crs:Sharpness="40"
   crs:SharpenRadius="+1.0"
   crs:SharpenDetail="25"
   crs:SharpenEdgeMasking="0"
   crs:LuminanceSmoothing="0"
   crs:ColorNoiseReduction="25"
   crs:ColorNoiseReductionDetail="50"
   crs:ColorNoiseReductionSmoothness="50"
   crs:HueAdjustmentRed="0"
   crs:HueAdjustmentOrange="0"
   crs:HueAdjustmentYellow="0"
   crs:HueAdjustmentGreen="0"
   crs:HueAdjustmentAqua="0"
   crs:HueAdjustmentBlue="0"
   crs:HueAdjustmentPurple="0"
   crs:HueAdjustmentMagenta="0"
   crs:SaturationAdjustmentRed="0"
   crs:SaturationAdjustmentOrange="+3"
   crs:SaturationAdjustmentYellow="0"
   crs:SaturationAdjustmentGreen="0"
   crs:SaturationAdjustmentAqua="0"
   crs:SaturationAdjustmentBlue="0"
   crs:SaturationAdjustmentPurple="0"
   crs:SaturationAdjustmentMagenta="0"
   crs:LuminanceAdjustmentRed="0"
   crs:LuminanceAdjustmentOrange="0"
   crs:LuminanceAdjustmentYellow="0"
   crs:LuminanceAdjustmentGreen="0"
   crs:LuminanceAdjustmentAqua="0"
   crs:LuminanceAdjustmentBlue="0"
   crs:LuminanceAdjustmentPurple="0"
   crs:LuminanceAdjustmentMagenta="0"
   crs:SplitToningShadowHue="259"
   crs:SplitToningShadowSaturation="23"
   crs:SplitToningHighlightHue="36"
   crs:SplitToningHighlightSaturation="24"
   crs:SplitToningBalance="0"
   crs:ColorGradeMidtoneHue="0"
   crs:ColorGradeMidtoneSat="0"
   crs:ColorGradeShadowLum="0"
   crs:ColorGradeMidtoneLum="0"
   crs:ColorGradeHighlightLum="0"
   crs:ColorGradeBlending="50"
   crs:ColorGradeGlobalHue="0"
   crs:ColorGradeGlobalSat="0"
   crs:ColorGradeGlobalLum="0"
   crs:AutoLateralCA="0"
   crs:LensProfileEnable="0"
   crs:LensManualDistortionAmount="0"
   crs:VignetteAmount="0"
   crs:DefringePurpleAmount="0"
   crs:DefringePurpleHueLo="30"
   crs:DefringePurpleHueHi="70"
   crs:DefringeGreenAmount="0"
   crs:DefringeGreenHueLo="40"
   crs:DefringeGreenHueHi="60"
   crs:PerspectiveUpright="0"
   crs:PerspectiveVertical="0"
   crs:PerspectiveHorizontal="0"
   crs:PerspectiveRotate="0.0"
   crs:PerspectiveAspect="0"
   crs:PerspectiveScale="100"
   crs:PerspectiveX="0.00"
   crs:PerspectiveY="0.00"
   crs:GrainAmount="0"
   crs:PostCropVignetteAmount="0"
   crs:ShadowTint="0"
   crs:RedHue="0"
   crs:RedSaturation="0"
   crs:GreenHue="0"
   crs:GreenSaturation="0"
   crs:BlueHue="0"
   crs:BlueSaturation="0"
   crs:OverrideLookVignette="False"
   crs:ToneCurveName2012="Linear"
   crs:CameraProfile="Adobe Standard"
   crs:CameraProfileDigest="2DE3C8E3E7A6454D52ADD2F110A1ADA2"
   crs:HasSettings="True"
   crs:CropTop="0"
   crs:CropLeft="0"
   crs:CropBottom="1"
   crs:CropRight="1"
   crs:CropAngle="0"
   crs:CropConstrainToWarp="0"
   crs:HasCrop="False"
   crs:AlreadyApplied="True"
   tiff:Make="NIKON CORPORATION"
   tiff:Model="NIKON D850"
   tiff:XResolution="300/1"
   tiff:YResolution="300/1"
   tiff:ResolutionUnit="2"
   exif:ExifVersion="0231"
   exif:ExposureTime="5/1"
   exif:ShutterSpeedValue="-2321928/1000000"
   exif:FNumber="4/1"
   exif:ApertureValue="4/1"
   exif:ExposureProgram="1"
   exif:SensitivityType="2"
   exif:RecommendedExposureIndex="500"
   exif:ExposureBiasValue="0/6"
   exif:MaxApertureValue="40/10"
   exif:MeteringMode="5"
   exif:LightSource="0"
   exif:FocalLength="700/10"
   exif:SensingMethod="2"
   exif:FileSource="3"
   exif:SceneType="1"
   exif:FocalLengthIn35mmFilm="70"
   exif:CustomRendered="0"
   exif:ExposureMode="1"
   exif:WhiteBalance="0"
   exif:SceneCaptureType="0"
   exif:GainControl="0"
   exif:Contrast="0"
   exif:Saturation="0"
   exif:Sharpness="0"
   exif:SubjectDistanceRange="0"
   exif:FocalPlaneXResolution="75409805/32768"
   exif:FocalPlaneYResolution="75409805/32768"
   exif:FocalPlaneResolutionUnit="3"
   exif:DateTimeOriginal="2025-11-03T21:11:27.63+08:00">
   <xmpMM:History>
    <rdf:Seq>
     <rdf:li
      stEvt:action="derived"
      stEvt:parameters="converted from image/x-nikon-nef to image/png, saved to new location"/>
     <rdf:li
      stEvt:action="saved"
      stEvt:instanceID="xmp.iid:fd2ea780-c31b-1042-8398-17f130a48fba"
      stEvt:when="2025-11-03T22:37:19+08:00"
      stEvt:softwareAgent="Adobe Photoshop Camera Raw 13.4 (Windows)"
      stEvt:changed="/"/>
    </rdf:Seq>
   </xmpMM:History>
   <xmpMM:DerivedFrom
    stRef:documentID="03FB35FD3EF347088E046579A6CBFEF4"
    stRef:originalDocumentID="03FB35FD3EF347088E046579A6CBFEF4"/>
   <crs:ToneCurvePV2012>
    <rdf:Seq>
     <rdf:li>0, 0</rdf:li>
     <rdf:li>255, 255</rdf:li>
    </rdf:Seq>
   </crs:ToneCurvePV2012>
   <crs:ToneCurvePV2012Red>
    <rdf:Seq>
     <rdf:li>0, 0</rdf:li>
     <rdf:li>255, 255</rdf:li>
    </rdf:Seq>
   </crs:ToneCurvePV2012Red>
   <crs:ToneCurvePV2012Green>
    <rdf:Seq>
     <rdf:li>0, 0</rdf:li>
     <rdf:li>255, 255</rdf:li>
    </rdf:Seq>
   </crs:ToneCurvePV2012Green>
   <crs:ToneCurvePV2012Blue>
    <rdf:Seq>
     <rdf:li>0, 0</rdf:li>
     <rdf:li>255, 255</rdf:li>
    </rdf:Seq>
   </crs:ToneCurvePV2012Blue>
   <crs:Look>
    <rdf:Description
     crs:Name="Adobe Color"
     crs:Amount="1.000000"
     crs:UUID="B952C231111CD8E0ECCF14B86BAA7077"
     crs:SupportsAmount="false"
     crs:SupportsMonochrome="false"
     crs:SupportsOutputReferred="false"
     crs:Copyright="© 2018 Adobe Systems, Inc.">
    <crs:Group>
     <rdf:Alt>
      <rdf:li xml:lang="x-default">Profiles</rdf:li>
     </rdf:Alt>
    </crs:Group>
    <crs:Parameters>
     <rdf:Description
      crs:Version="13.3"
      crs:ProcessVersion="11.0"
      crs:ConvertToGrayscale="False"
      crs:CameraProfile="Adobe Standard"
      crs:LookTable="E1095149FDB39D7A057BAB208837E2E1">
     <crs:ToneCurvePV2012>
      <rdf:Seq>
       <rdf:li>0, 0</rdf:li>
       <rdf:li>22, 16</rdf:li>
       <rdf:li>40, 35</rdf:li>
       <rdf:li>127, 127</rdf:li>
       <rdf:li>224, 230</rdf:li>
       <rdf:li>240, 246</rdf:li>
       <rdf:li>255, 255</rdf:li>
      </rdf:Seq>
     </crs:ToneCurvePV2012>
     <crs:ToneCurvePV2012Red>
      <rdf:Seq>
       <rdf:li>0, 0</rdf:li>
       <rdf:li>255, 255</rdf:li>
      </rdf:Seq>
     </crs:ToneCurvePV2012Red>
     <crs:ToneCurvePV2012Green>
      <rdf:Seq>
       <rdf:li>0, 0</rdf:li>
       <rdf:li>255, 255</rdf:li>
      </rdf:Seq>
     </crs:ToneCurvePV2012Green>
     <crs:ToneCurvePV2012Blue>
      <rdf:Seq>
       <rdf:li>0, 0</rdf:li>
       <rdf:li>255, 255</rdf:li>
      </rdf:Seq>
     </crs:ToneCurvePV2012Blue>
     </rdf:Description>
    </crs:Parameters>
    </rdf:Description>
   </crs:Look>
   <exif:ISOSpeedRatings>
    <rdf:Seq>
     <rdf:li>500</rdf:li>
    </rdf:Seq>
   </exif:ISOSpeedRatings>
   <exif:Flash
    exif:Fired="False"
    exif:Return="0"
    exif:Mode="0"
    exif:Function="False"
    exif:RedEyeMode="False"/>
  </rdf:Description>
 </rdf:RDF>
</x:xmpmeta>

<?xpacket end="w"?>
```
---

##### metadata


#### 合并远程仓库内容
- 先创建密钥
- `git remote add origin https://你的用户名:你的令牌@github.com/BewiZ/Rust_for_exif.git`
- `git pull origin main --allow-unrelated-histories`拉取远程内容并允许不相关的历史记录
- `git add .`添加所有文件到暂存区
- `git commit -m "合并远程仓库内容"`提交合并
- `git branch -M main`创建 main 分支
- `git push -u origin main`推送代码
