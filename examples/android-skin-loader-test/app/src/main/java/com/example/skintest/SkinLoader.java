package com.example.skintest;

import android.content.res.AssetManager;
import android.content.res.Resources;
import android.util.Log;

import java.lang.reflect.Method;

/**
 * 皮肤加载器 - 演示如何正确加载 ASB 生成的皮肤包
 * 
 * 关键点：
 * 1. 使用 AssetManager.addAssetPath() 加载皮肤包
 * 2. 检查返回的 cookie 值（0 表示失败）
 * 3. 使用皮肤包的包名（不是应用包名）
 * 4. 确保 ASB 配置中设置了 packageId = "0x7f"
 */
public class SkinLoader {
    private static final String TAG = "SkinLoader";
    
    private Resources skinResources;
    private String skinPackageName;
    
    /**
     * 加载皮肤包
     * 
     * @param skinPath 皮肤包文件路径
     * @param hostResources 宿主应用的 Resources
     * @param skinPackageName 皮肤包的包名（必须与 ASB 配置中的 packageName 一致）
     * @return 是否加载成功
     */
    public boolean loadSkin(String skinPath, Resources hostResources, String skinPackageName) {
        this.skinPackageName = skinPackageName;
        
        try {
            // 1. 创建新的 AssetManager 实例
            AssetManager assetManager = AssetManager.class.newInstance();
            
            // 2. 使用反射调用 addAssetPath 方法添加皮肤包路径
            Method addAssetPath = AssetManager.class.getMethod("addAssetPath", String.class);
            Object cookie = addAssetPath.invoke(assetManager, skinPath);
            
            // 3. 检查 cookie 值 - 0 表示加载失败
            if (cookie == null || (int) cookie == 0) {
                Log.e(TAG, "Failed to add asset path: " + skinPath);
                Log.e(TAG, "Cookie value: " + cookie);
                return false;
            }
            
            Log.d(TAG, "Successfully added asset path: " + skinPath);
            Log.d(TAG, "Cookie value: " + cookie);
            
            // 4. 创建 Resources 对象
            // 使用宿主应用的 DisplayMetrics 和 Configuration 确保兼容性
            skinResources = new Resources(
                assetManager,
                hostResources.getDisplayMetrics(),
                hostResources.getConfiguration()
            );
            
            Log.d(TAG, "Skin Resources created successfully");
            
            // 5. 验证资源是否可以访问
            // 尝试获取一个已知的资源 ID
            int testColorId = skinResources.getIdentifier("primary_color", "color", skinPackageName);
            Log.d(TAG, "Test resource ID (primary_color): 0x" + Integer.toHexString(testColorId));
            
            if (testColorId == 0) {
                Log.e(TAG, "Failed to get resource ID for 'primary_color'");
                Log.e(TAG, "Package name used: " + skinPackageName);
                Log.e(TAG, "This usually means:");
                Log.e(TAG, "  1. ASB packageId is not set to 0x7f");
                Log.e(TAG, "  2. Package name doesn't match skin package");
                Log.e(TAG, "  3. Resource doesn't exist in skin package");
                return false;
            }
            
            return true;
            
        } catch (Exception e) {
            Log.e(TAG, "Error loading skin", e);
            return false;
        }
    }
    
    /**
     * 获取皮肤资源中的颜色
     * 
     * @param resourceName 资源名称（如 "primary_color"）
     * @param defaultColor 默认颜色（如果获取失败）
     * @return 颜色值
     */
    public int getColor(String resourceName, int defaultColor) {
        if (skinResources == null || skinPackageName == null) {
            Log.w(TAG, "Skin not loaded, returning default color");
            return defaultColor;
        }
        
        try {
            // 使用 getIdentifier 获取资源 ID
            // 注意：必须使用皮肤包的包名
            int colorId = skinResources.getIdentifier(resourceName, "color", skinPackageName);
            
            if (colorId == 0) {
                Log.w(TAG, "Resource not found: " + resourceName);
                return defaultColor;
            }
            
            // 获取颜色值
            // 注意：API 23+ 需要传入 theme 参数（可以为 null）
            int color = skinResources.getColor(colorId, null);
            
            Log.d(TAG, "Got color " + resourceName + ": 0x" + Integer.toHexString(color) + 
                      " (ID: 0x" + Integer.toHexString(colorId) + ")");
            
            return color;
            
        } catch (Exception e) {
            Log.e(TAG, "Error getting color: " + resourceName, e);
            return defaultColor;
        }
    }
    
    /**
     * 获取皮肤资源中的字符串
     * 
     * @param resourceName 资源名称
     * @param defaultValue 默认值
     * @return 字符串值
     */
    public String getString(String resourceName, String defaultValue) {
        if (skinResources == null || skinPackageName == null) {
            return defaultValue;
        }
        
        try {
            int stringId = skinResources.getIdentifier(resourceName, "string", skinPackageName);
            
            if (stringId == 0) {
                Log.w(TAG, "String resource not found: " + resourceName);
                return defaultValue;
            }
            
            return skinResources.getString(stringId);
            
        } catch (Exception e) {
            Log.e(TAG, "Error getting string: " + resourceName, e);
            return defaultValue;
        }
    }
    
    /**
     * 获取资源 ID（用于调试）
     * 
     * @param resourceName 资源名称
     * @param resourceType 资源类型（如 "color", "string", "drawable"）
     * @return 资源 ID，如果不存在返回 0
     */
    public int getResourceId(String resourceName, String resourceType) {
        if (skinResources == null || skinPackageName == null) {
            return 0;
        }
        
        return skinResources.getIdentifier(resourceName, resourceType, skinPackageName);
    }
    
    /**
     * 获取皮肤包的 Resources 对象
     */
    public Resources getResources() {
        return skinResources;
    }
    
    /**
     * 检查是否已加载皮肤
     */
    public boolean isLoaded() {
        return skinResources != null;
    }
    
    /**
     * 释放皮肤资源
     */
    public void release() {
        skinResources = null;
        skinPackageName = null;
        Log.d(TAG, "Skin resources released");
    }
}
